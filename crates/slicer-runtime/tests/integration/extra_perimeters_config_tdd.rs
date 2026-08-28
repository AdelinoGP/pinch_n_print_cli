//! AC-1 (packet 108, T-070/T-071): `extra_perimeters` per-region config bonus.
//!
//! OrcaSlicer PerimeterGenerator.cpp:1569 —
//! `int loop_number = this->config->wall_loops + surface.extra_perimeters - 1;
//! // 0-indexed loops`
//!
//! A region with base `wall_count=2` and `extra_perimeters=2` must emit exactly
//! 4 walls (loop_number = wall_count + extra_perimeters - 1, zero-indexed);
//! with `extra_perimeters=0` it must emit exactly 2 walls (bonus is a no-op).

use classic_perimeters::ClassicPerimeters;
use slicer_ir::LoopType;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

/// Run perimeters with the given config and return emitted Outer/Inner wall loops.
fn run_with_config(config: slicer_ir::ConfigView) -> Vec<slicer_ir::WallLoop> {
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
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

/// AC-1 positive case: base wall_count=2, extra_perimeters=2 → 4 walls.
#[test]
fn extra_perimeters_bonus_adds_to_wall_count() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .int("extra_perimeters", 2)
        .build();

    let walls = run_with_config(config);
    assert_eq!(
        walls.len(),
        4,
        "Expected 4 wall loops (wall_count=2 + extra_perimeters=2); got {}",
        walls.len()
    );
}

/// AC-1 no-op case: base wall_count=2, extra_perimeters=0 → 2 walls (unchanged).
#[test]
fn extra_perimeters_zero_is_noop() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .int("extra_perimeters", 0)
        .build();

    let walls = run_with_config(config);
    assert_eq!(
        walls.len(),
        2,
        "Expected 2 wall loops (wall_count=2 + extra_perimeters=0); got {}",
        walls.len()
    );
}

// ---------------------------------------------------------------------------
// Packet 212 — arachne parity for `extra_perimeters` (AC-1/AC-2/AC-3/AC-N1/AC-N2)
//
// `arachne-perimeters` auto-derives `max_bead_count = 2 * wall_count` in
// `arachne_params_from_config` and never reads `extra_perimeters`, so switching
// `wall_generator` silently discards the bonus walls. The fixture below is the
// 20 mm square / 1.0 mm bead fixture from
// `modules/core-modules/arachne-perimeters/tests/alternate_extra_wall_tdd.rs`,
// whose measured mapping is: for an EVEN `max_bead_count`, the emitted wall
// count is exactly `max_bead_count / 2` (`LimitedBeadingStrategy`'s symmetric
// sentinel pair is filtered as zero-width by `remove_small_lines`).
// ---------------------------------------------------------------------------

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::ConfigView;

/// 1.0 mm beads on a 20 mm square: the bead cap is always the binding
/// constraint, never the polygon's own geometric bead capacity.
const BEAD_WIDTH_MM: f64 = 1.0;
const SQUARE_SIDE_MM: f32 = 20.0;

/// Shared arachne/classic fixture region (20 mm square).
fn make_square_region() -> SliceRegionView {
    make_region(SQUARE_SIDE_MM, 0.2)
}

/// Emitted Outer/Inner wall-loop count from `ArachnePerimeters`.
fn arachne_wall_count(config: &ConfigView, layer_index: u32) -> usize {
    let module = ArachnePerimeters::from_config(config).unwrap();
    let regions = vec![make_square_region()];
    let paint = PaintRegionLayerView::new(layer_index);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(layer_index, &regions, &paint, &mut output, config)
        .unwrap();
    // Arachne's emit path also pushes ThinWall / GapFill loops (see
    // `classify_line` in `modules/core-modules/arachne-perimeters/src/lib.rs`),
    // so filter to the same Outer/Inner set `classic_wall_count` counts.
    output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer || w.loop_type == LoopType::Inner)
        .count()
}

/// Emitted Outer/Inner wall-loop count from `ClassicPerimeters` on the SAME
/// fixture, so AC-3 compares like with like.
fn classic_wall_count(config: &ConfigView, layer_index: u32) -> usize {
    let module = ClassicPerimeters::from_config(config).unwrap();
    let regions = vec![make_square_region()];
    let paint = PaintRegionLayerView::new(layer_index);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(layer_index, &regions, &paint, &mut output, config)
        .unwrap();
    output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer || w.loop_type == LoopType::Inner)
        .count()
}

/// `wall_count=2`, `extra_perimeters=<n>`, NO `max_bead_count` key — the
/// auto-derivation path that must fold the bonus in.
fn arachne_config(extra_perimeters: i64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("inner_wall_line_width", BEAD_WIDTH_MM)
        .float("outer_wall_line_width", BEAD_WIDTH_MM)
        .int("wall_count", 2)
        .int("extra_perimeters", extra_perimeters)
        .build()
}

/// AC-1: arachne must fold `extra_perimeters` into the auto-derived cap.
/// `max_bead_count = 2 * (wall_count + extra_perimeters) = 2 * (2 + 2) = 8`,
/// and the measured even-cap mapping gives `8 / 2 = 4` emitted walls.
#[test]
fn arachne_extra_perimeters_bonus_adds_to_wall_count() {
    let config = arachne_config(2);
    let walls = arachne_wall_count(&config, 0);
    assert_eq!(
        walls, 4,
        "arachne must emit 4 wall loops for wall_count=2 + extra_perimeters=2 \
         (auto max_bead_count = 2*(2+2) = 8, emitted = 8/2); got {walls}"
    );
}

/// AC-2: `extra_perimeters = 0` is a no-op — the auto cap stays `2 * 2 = 4`
/// and emits 2 walls.
#[test]
fn arachne_extra_perimeters_zero_is_noop() {
    let config = arachne_config(0);
    let walls = arachne_wall_count(&config, 0);
    assert_eq!(
        walls, 2,
        "arachne with extra_perimeters=0 must emit the unchanged 2 wall loops \
         (auto max_bead_count = 2*2 = 4, emitted = 4/2); got {walls}"
    );
}

/// AC-3: one shared config, both generators — switching `wall_generator` must
/// not change how many walls the user gets.
#[test]
fn extra_perimeters_survives_wall_generator_switch() {
    let config = arachne_config(2);

    let classic_count = classic_wall_count(&config, 0);
    let arachne_count = arachne_wall_count(&config, 0);

    assert_eq!(
        classic_count, arachne_count,
        "wall_count=2 + extra_perimeters=2 must emit the same wall-loop count \
         under both generators; classic={classic_count}, arachne={arachne_count}"
    );
    assert_eq!(
        arachne_count, 4,
        "both generators must emit 4 wall loops for wall_count=2 + \
         extra_perimeters=2; got {arachne_count}"
    );
}

/// AC-N1 (negative): an EXPLICIT positive `max_bead_count` is an advanced
/// override honoured verbatim — `extra_perimeters` must NOT inflate it.
/// `max_bead_count = 4` -> 2 emitted walls regardless of the bonus.
#[test]
fn arachne_explicit_max_bead_count_override_ignores_extra_perimeters() {
    let config = ConfigViewBuilder::new()
        .float("inner_wall_line_width", BEAD_WIDTH_MM)
        .float("outer_wall_line_width", BEAD_WIDTH_MM)
        .int("wall_count", 2)
        .int("extra_perimeters", 2)
        .int("max_bead_count", 4)
        .build();

    let walls = arachne_wall_count(&config, 0);
    assert_eq!(
        walls, 2,
        "an explicit max_bead_count=4 is an advanced override honoured \
         verbatim; extra_perimeters must not inflate it (expected 2); got {walls}"
    );
}

/// AC-N2 (composition): the `extra_perimeters` fold happens inside
/// `arachne_params_from_config`, i.e. BEFORE `run_perimeters`'s
/// `params.max_bead_count += 2` odd-layer `alternate_extra_wall` bump, so the
/// two compose additively: `2 * (2 + 2) = 8`, `+2` on the odd layer = `10`,
/// emitted = `10 / 2` = 5.
#[test]
fn arachne_extra_perimeters_composes_with_alternate_extra_wall() {
    let config = ConfigViewBuilder::new()
        .float("inner_wall_line_width", BEAD_WIDTH_MM)
        .float("outer_wall_line_width", BEAD_WIDTH_MM)
        .int("wall_count", 2)
        .int("extra_perimeters", 2)
        .bool("alternate_extra_wall", true)
        .bool("spiral_vase", false)
        .float("sparse_infill_density", 20.0)
        .build();

    let walls = arachne_wall_count(&config, 1);
    assert_eq!(
        walls, 5,
        "extra_perimeters must compose additively with the odd-layer \
         alternate_extra_wall bump (max_bead_count = 2*(2+2) + 2 = 10, \
         emitted = 10/2); got {walls}"
    );
}
