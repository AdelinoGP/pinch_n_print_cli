//! `min_bead_width` must default to canonical 85% of the nozzle diameter.
//!
//! OrcaSlicer `PrintConfig.cpp` registers `min_bead_width` as `coPercent` with
//! default 85; `WallToolPaths.cpp` derives `wall_split_middle_threshold =
//! clamp(2 * min_bead_width / external_perimeter_width - 1, 0.01, 0.99)`,
//! which places the 1 -> 2 bead transition of a thin strip. With a 100%
//! default the threshold clamps to 0.99 and strips up to ~0.8 mm print as a
//! single open centre bead instead of a closed two-bead loop.
//!
//! Oracle: OrcaSlicer 2.4.1 CLI, "0.20mm Standard @BBL X1C" with Arachne
//! (0.42 outer / 0.45 inner widths, 0.4 nozzle, min_bead_width 85%), on
//! 12 mm-long strips: 0.62 mm -> one open 11.38 mm outer line; 0.70 mm -> one
//! closed 23.96 mm outer loop.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::ConfigView;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};

fn config() -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 5)
        .float("outer_wall_line_width", 0.42)
        .float("inner_wall_line_width", 0.45)
        .float("nozzle_diameter", 0.4)
        .build()
}

/// Walls of a `width_mm` x 12 mm strip at layer 3 (not the initial layer).
fn strip_walls(width_mm: f32) -> Vec<slicer_ir::WallLoop> {
    let config = config();
    let module = ArachnePerimeters::from_config(&config).unwrap();
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.8)
        .add_polygon(rect_polygon(0.0, 0.0, 12.0, width_mm))
        .build();
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(3, &[region], &PaintRegionLayerView::new(3), &mut output, &config)
        .unwrap();
    output.wall_loops().to_vec()
}

#[test]
fn strip_above_canonical_split_threshold_prints_closed_two_bead_loop() {
    let walls = strip_walls(0.70);
    assert_eq!(
        walls.len(),
        1,
        "a 0.70 mm strip must print one wall line; got {} lines",
        walls.len()
    );
    assert!(
        walls[0].path.is_closed(),
        "a 0.70 mm strip is above the canonical 1->2 bead transition (~0.68 mm): \
         two beads meet at the strip ends and form one closed loop, not an open \
         centre bead"
    );
}

#[test]
fn strip_below_canonical_split_threshold_prints_open_centre_bead() {
    let walls = strip_walls(0.62);
    assert_eq!(walls.len(), 1, "a 0.62 mm strip must print one wall line");
    assert!(
        !walls[0].path.is_closed(),
        "a 0.62 mm strip is below the canonical 1->2 bead transition: one open \
         centre bead"
    );
}
