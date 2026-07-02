//! AC-2 (packet 108, T-072/T-073): narrow-island smaller-width override.
//!
//! Loosely follows OrcaSlicer PerimeterGenerator.cpp:1611-1628's "narrow but
//! not too long" island classification: an island whose narrow-dimension
//! opening erodes to nothing gets its outer wall generated at the narrower
//! `smaller_perimeter_line_width` instead of `outer_wall_line_width`.
//!
//! Two acceptance cases:
//!  - a long narrow rect island (20 mm × 0.6 mm) with
//!    `smaller_perimeter_threshold_mm=0.8` and `smaller_perimeter_line_width=0.3`
//!    must emit its outer wall with per-vertex width 0.3 mm.
//!  - a wider island (20 mm × 5 mm) in the same fixture must keep the default
//!    `outer_wall_line_width` on its outer wall.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::LoopType;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_rect_region(region_id: u64, width_mm: f32, height_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(region_id)
        .z(z)
        .add_polygon(rect_polygon(0.0, 0.0, width_mm, height_mm))
        .build()
}

/// Run perimeters with the given config for a single fixture region and
/// return the per-vertex widths of its Outer wall loop(s).
fn outer_wall_widths_for(config: &slicer_ir::ConfigView, region: &SliceRegionView) -> Vec<f32> {
    let module = ClassicPerimeters::on_print_start(config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, std::slice::from_ref(region), &paint, &mut output, config)
        .unwrap();
    output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer)
        .flat_map(|w| w.width_profile.widths.clone())
        .collect()
}

/// AC-2 positive case: narrow island (20 mm × 0.6 mm) uses
/// `smaller_perimeter_line_width` on its outer wall.
#[test]
fn narrow_island_uses_smaller_perimeter_width() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", 0.5)
        .float("smaller_perimeter_threshold_mm", 0.8)
        .float("smaller_perimeter_line_width", 0.3)
        .build();

    let narrow_region = make_rect_region(1, 20.0, 0.6, 0.2);
    let narrow_widths = outer_wall_widths_for(&config, &narrow_region);
    assert!(
        !narrow_widths.is_empty(),
        "expected at least one outer-wall vertex width for the narrow island"
    );
    for w in &narrow_widths {
        assert!(
            (*w - 0.3).abs() < 1e-3,
            "expected narrow island outer wall width ~0.3 mm, got {w}"
        );
    }
}

/// AC-2 negative case: a wider island (20 mm × 5 mm) in the same fixture
/// keeps the default `outer_wall_line_width` (0.5 mm) on its outer wall.
#[test]
fn wide_island_keeps_default_outer_wall_width() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", 0.5)
        .float("smaller_perimeter_threshold_mm", 0.8)
        .float("smaller_perimeter_line_width", 0.3)
        .build();

    let wide_region = make_rect_region(2, 20.0, 5.0, 0.2);
    let wide_widths = outer_wall_widths_for(&config, &wide_region);
    assert!(
        !wide_widths.is_empty(),
        "expected at least one outer-wall vertex width for the wide island"
    );
    for w in &wide_widths {
        assert!(
            (*w - 0.5).abs() < 1e-3,
            "expected wide island outer wall width ~0.5 mm (default), got {w}"
        );
    }
}
