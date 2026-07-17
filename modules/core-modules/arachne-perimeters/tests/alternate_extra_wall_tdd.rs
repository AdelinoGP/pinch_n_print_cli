//! TDD test for packet 149, AC-3: `alternate_extra_wall`.
//!
//! OrcaSlicer's `alternate_extra_wall` adds one extra wall loop on every
//! second (odd) layer by incrementing `loop_number` before constructing
//! `WallToolPaths` (`PrintConfig.cpp:5059-5066`), which maps to
//! `max_bead_count = 2 * inset_count` inside the beading-strategy factory.
//! This test drives `ArachnePerimeters::run_perimeters` end-to-end and
//! asserts the emitted wall-loop *count* actually grows on odd layers when
//! the gate is satisfied, and stays flat when `alternate_extra_wall` is off.
//!
//! The fixture pins the wall count via `max_bead_count` directly (not the
//! module's vestigial, unread `wall_count` config key — see
//! `precise_outer_wall_tdd.rs`'s own `make_config` for that key's no-op
//! status in this module) on a square large enough that the geometry always
//! has far more beading headroom than the cap, so the emitted wall-loop
//! count is fully determined by `max_bead_count`, deterministically.
//!
//! **Empirically measured mapping** (probed against this exact fixture
//! before wiring the production bump): `LimitedBeadingStrategy` inserts a
//! symmetric sentinel pair (`beading/limited.rs`'s own doc comment) that
//! `remove_small_lines` then filters as zero-width, so the emitted wall
//! count is NOT `max_bead_count` verbatim — for an *even* `max_bead_count`
//! it is exactly `max_bead_count / 2`. `max_bead_count = 4` -> 2 walls,
//! `max_bead_count = 6` -> 3 walls (holding parity even keeps the mapping
//! linear: `+2` to `max_bead_count` == `+1` emitted wall). This is exactly
//! why the production bump in `run_perimeters` adds 2, not 1, to
//! `max_bead_count` — mirroring OrcaSlicer's own `max_bead_count = 2 *
//! inset_count` relation (one extra `inset_count` step == two
//! `max_bead_count` units == one extra emitted wall here).

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::ConfigView;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

const BEAD_WIDTH_MM: f32 = 1.0;
/// Large enough that a 4/6-bead cap is always the binding constraint, not
/// the polygon's own geometric bead capacity.
const SQUARE_SIDE_MM: f32 = 20.0;
/// Chosen (even) so the emitted wall count is exactly `max_bead_count / 2`
/// == 2 walls at baseline (see module doc comment for the measured mapping).
const BASE_MAX_BEAD_COUNT: i64 = 4;
const BASE_WALL_COUNT: usize = 2;
const BUMPED_WALL_COUNT: usize = 3;

fn make_config(alternate_extra_wall: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .float("inner_wall_line_width", BEAD_WIDTH_MM as f64)
        .float("outer_wall_line_width", BEAD_WIDTH_MM as f64)
        .int("max_bead_count", BASE_MAX_BEAD_COUNT)
        .bool("alternate_extra_wall", alternate_extra_wall)
        .bool("spiral_vase", false)
        .float("sparse_infill_density", 20.0)
        .build()
}

fn make_region(z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, SQUARE_SIDE_MM))
        .build()
}

fn wall_loop_count(config: &ConfigView, layer_index: u32) -> usize {
    let module = ArachnePerimeters::on_print_start(config).unwrap();
    let regions = vec![make_region(0.2)];
    let paint = PaintRegionLayerView::new(layer_index);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(layer_index, &regions, &paint, &mut output, config)
        .unwrap();

    output.wall_loops().len()
}

/// AC-3 (positive): with `alternate_extra_wall=true`, an even layer
/// (layer_index 0) emits `BASE_WALL_COUNT` (2) walls, and an odd layer
/// (layer_index 1) emits one more (`BUMPED_WALL_COUNT`, 3) — the
/// `max_bead_count += 2` beading-stack bump described in
/// `arachne-perimeters/src/lib.rs`'s `run_perimeters`.
#[test]
fn alternate_extra_wall_bumps_wall_count_on_odd_layers() {
    let config = make_config(true);

    let even_count = wall_loop_count(&config, 0);
    let odd_count = wall_loop_count(&config, 1);

    assert_eq!(
        even_count, BASE_WALL_COUNT,
        "even layer (layer_index 0) with alternate_extra_wall=true must \
         emit exactly {BASE_WALL_COUNT} walls (max_bead_count={BASE_MAX_BEAD_COUNT}); \
         got {even_count}"
    );
    assert_eq!(
        odd_count, BUMPED_WALL_COUNT,
        "odd layer (layer_index 1) with alternate_extra_wall=true must emit \
         one extra wall beyond the even-layer baseline ({BASE_WALL_COUNT} -> \
         {BUMPED_WALL_COUNT}); got {odd_count} (even layer emitted {even_count})"
    );
}

/// AC-3 (negative, gate honesty): with `alternate_extra_wall=false`, an odd
/// layer must NOT gain an extra wall — it stays at `BASE_WALL_COUNT` (2),
/// same as the even layer.
#[test]
fn alternate_extra_wall_off_keeps_odd_layer_wall_count_flat() {
    let config = make_config(false);

    let even_count = wall_loop_count(&config, 0);
    let odd_count = wall_loop_count(&config, 1);

    assert_eq!(
        odd_count, BASE_WALL_COUNT,
        "odd layer (layer_index 1) with alternate_extra_wall=false must emit \
         the same {BASE_WALL_COUNT} walls as the even layer; got {odd_count} \
         (even layer emitted {even_count})"
    );
    assert_eq!(
        even_count, odd_count,
        "alternate_extra_wall=false must produce identical wall counts on \
         even and odd layers; even={even_count}, odd={odd_count}"
    );
}
