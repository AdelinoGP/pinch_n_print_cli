//! TDD test for arachne per-vertex parity packet 148, AC-1.
//!
//! OrcaSlicer's Arachne path assigns the outermost bead (`inset_idx == 0`)
//! the `ExteriorSurface` boundary type — it faces air or a gap, exactly like
//! `classic-perimeters`' own outer wall. This module previously hardcoded
//! `WallBoundaryType::Interior` for every emitted `WallLoop` regardless of
//! `perimeter_index`, which is wrong for the outermost wall.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, WallBoundaryType};
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
        .float("inner_wall_line_width", line_width_mm as f64)
        .float("outer_wall_line_width", line_width_mm as f64)
        // Required-read baseline (packet 06 5c', item 11): the classified
        // reads in run_perimeters/arachne_params_from_config are now
        // require_*; the view holds every key those paths read, at
        // manifest-default values. line_width holds its post-expansion
        // default (1.125 x nozzle_diameter): the raw 0 is the auto sentinel
        // expanded at Phase B and cannot survive the D-162 spacing gate.
        .float("layer_height", 0.2)
        .float("nozzle_diameter", 0.4)
        .float("line_width", 0.45)
        .float("bridge_line_width", 0.0)
        .float("initial_layer_line_width", 0.0)
        .int("extra_perimeters", 0)
        .bool("precise_outer_wall", false)
        .string("wall_sequence", "InnerOuter")
        .int("support_raft_layers", 0)
        .bool("only_one_wall_top", false)
        .string("wall_direction", "counter_clockwise")
        .bool("alternate_extra_wall", false)
        .bool("spiral_vase", false)
        .float("sparse_infill_density", 20.0)
        .bool("only_one_wall_first_layer", false)
        .bool("detect_overhang_wall", true)
        .bool("overhang_reverse", false)
        .bool("overhang_reverse_internal_only", false)
        .float("overhang_reverse_threshold", 0.0)
        .float("bridge_flow", 1.0)
        .bool("thick_bridges", false)
        .float("seam_candidate_angle_threshold_deg", 30.0)
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
    let module = ArachnePerimeters::from_config(&config).unwrap();
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
