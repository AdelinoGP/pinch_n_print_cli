//! TDD test for arachne per-vertex parity packet 148, AC-2.
//!
//! OrcaSlicer's Arachne path emits widened center-line beads (an `is_odd`
//! bead at `inset_idx == 0`, produced by `WideningBeadingStrategy` only when
//! `print_thin_walls` is on) as `erExternalPerimeter`
//! (`PerimeterGenerator.cpp:383-384`) — the Arachne skeletal-graph algorithm
//! itself has no thin-wall/gap-fill role; `is_odd` is purely structural
//! (center-line beads). `LoopType::ThinWall` here is PnP's own semantic
//! refinement of that same widened center-line bead: a region thinner than
//! one full bead, only produced when thin-wall detection (`detect_thin_wall`
//! → `ArachneParams::print_thin_walls`) is enabled.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, LoopType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Config with a nominal 0.4mm optimal/outer bead width and thin-wall
/// detection toggled per `detect_thin_wall_on`.
///
/// `optimal_width`/`preferred_bead_width_outer` are `unit = "units"` keys
/// (1 unit = 100 nm), so the mm width is converted via [`mm_to_units`]
/// before being stored (see `arachne-perimeters.toml`).
fn make_config(detect_thin_wall_on: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("inner_wall_line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
        .bool("detect_thin_wall", detect_thin_wall_on)
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

/// A 0.25mm x 5mm thin strip, narrower than one full 0.4mm bead
/// (`min_bead_width` default) but wider than `min_feature_size` (default
/// 0.1mm) — thin enough that `WideningBeadingStrategy` widens it to a single
/// odd center-line bead when `print_thin_walls` is on.
fn make_thin_strip_region(z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(rect_polygon(0.0, 0.0, 0.25, 5.0))
        .build()
}

#[test]
fn thin_strip_with_detection_on_emits_thin_wall_loop_type() {
    let config = make_config(true);
    let module = ArachnePerimeters::from_config(&config).unwrap();
    let regions = vec![make_thin_strip_region(0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(
        !walls.is_empty(),
        "the thin-strip fixture must emit at least one wall loop"
    );
    assert!(
        walls.iter().any(|w| w.loop_type == LoopType::ThinWall),
        "a 0.25mm-wide strip with detect_thin_wall=true must emit at least one \
         WallLoop with loop_type == LoopType::ThinWall; got loop_types: {:?}",
        walls.iter().map(|w| w.loop_type).collect::<Vec<_>>()
    );
}
