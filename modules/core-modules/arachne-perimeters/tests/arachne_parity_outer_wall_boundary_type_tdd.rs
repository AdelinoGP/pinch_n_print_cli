//! TDD test for arachne per-vertex parity packet 148, AC-1.
//!
//! OrcaSlicer's Arachne path assigns the outermost bead (`inset_idx == 0`)
//! the `ExteriorSurface` boundary type — it faces air or a gap, exactly like
//! `classic-perimeters`' own outer wall. This module previously hardcoded
//! `WallBoundaryType::Interior` for every emitted `WallLoop` regardless of
//! `perimeter_index`, which is wrong for the outermost wall.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{mm_to_units, ConfigView, WallBoundaryType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Create a config with wall_count and line_width, enough for Arachne to
/// produce a multi-wall bead sequence.
///
/// `optimal_width`/`preferred_bead_width_outer` are `unit = "units"` keys
/// (1 unit = 100 nm, see `arachne-perimeters.toml`), so `line_width` (given
/// in mm) must be converted via [`mm_to_units`] before being stored — unlike
/// classic-perimeters' `line_width` key, which is read as a bare mm float.
fn make_config(wall_count: u32, line_width_mm: f32) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("optimal_width", mm_to_units(line_width_mm) as f64)
        .float(
            "preferred_bead_width_outer",
            mm_to_units(line_width_mm) as f64,
        )
        .build()
}

/// 10mm square region, per the packet's AC-1 fixture description.
fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

#[test]
fn outer_wall_has_exterior_surface_boundary_type() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let outer_wall = output
        .wall_loops()
        .iter()
        .find(|w| w.perimeter_index == 0)
        .expect("a wall loop with perimeter_index == 0 must be emitted");

    assert_eq!(
        outer_wall.boundary_type,
        WallBoundaryType::ExteriorSurface,
        "the perimeter_index == 0 wall loop (outermost bead, facing air) must have \
         boundary_type == ExteriorSurface, got {:?}",
        outer_wall.boundary_type
    );
}
