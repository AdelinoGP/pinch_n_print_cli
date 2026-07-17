//! TDD test for arachne per-vertex parity packet 148, AC-3.
//!
//! `WallFeatureFlags::is_thin_wall` must be set on every vertex of a
//! `LoopType::ThinWall` wall (the widened `is_odd`/`inset_idx == 0` center-line
//! bead — see `arachne_parity_thin_wall_loop_type_tdd.rs` and this module's
//! `classify_line` doc comment for the OrcaSlicer provenance) and must never
//! be set on `Outer`/`Inner` walls, even if geometrically narrow — mirroring
//! `classic-perimeters`' own `is_thin_wall` flag shape (only set on its
//! `ThinWall` loop-type walls).

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
fn is_thin_wall_flag_set_only_on_thin_wall_loops() {
    let config = make_config(true);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
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

    let thin_wall_loops: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::ThinWall)
        .collect();
    assert!(
        !thin_wall_loops.is_empty(),
        "the thin-strip fixture must emit at least one LoopType::ThinWall wall \
         (see arachne_parity_thin_wall_loop_type_tdd.rs); got loop_types: {:?}",
        walls.iter().map(|w| w.loop_type).collect::<Vec<_>>()
    );
    for wall in &thin_wall_loops {
        assert!(
            wall.feature_flags.iter().all(|f| f.is_thin_wall),
            "every vertex's WallFeatureFlags on a LoopType::ThinWall wall must have \
             is_thin_wall == true; got {:?}",
            wall.feature_flags
                .iter()
                .map(|f| f.is_thin_wall)
                .collect::<Vec<_>>()
        );
    }

    // Negative shape (AC-3): is_thin_wall must never be set on Outer/Inner
    // walls, even if geometrically narrow.
    for wall in walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer || w.loop_type == LoopType::Inner)
    {
        assert!(
            wall.feature_flags.iter().all(|f| !f.is_thin_wall),
            "Outer/Inner walls must never have is_thin_wall == true (loop_type {:?}); \
             got {:?}",
            wall.loop_type,
            wall.feature_flags
                .iter()
                .map(|f| f.is_thin_wall)
                .collect::<Vec<_>>()
        );
    }
}
