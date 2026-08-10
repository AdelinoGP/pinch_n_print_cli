//! Regression lock for the beading spacing -> extrusion width conversion
//! (D-160 / PNP_KNOWN_ISSUES item 6).
//!
//! Arachne's beading engine works in Flow SPACING (canonical feeds
//! `ext_perimeter_spacing` as `bead_width_0`), so the emitted path widths are
//! converted back at the ExtrusionLine -> path boundary
//! (`build_walls`'s `flow_to_width(pt.width, layer_height_mm)`). These tests
//! pin the emitted widths to the CONFIGURED values (0.525 mm outer, 0.625 mm
//! inner), for the plain path and the first layer's
//! `initial_layer_line_width`, so a future regression cannot silently emit
//! spacing-as-width again.
//!
//! (The measured "outer wall ~0.50 mm at 0.525 configured" in PNP_KNOWN_ISSUES
//! item 6 was traced to bridge-flow vertices: 0.525 × bridge_flow(0.95) =
//! 0.49875 — the conversion itself is exact.)

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, LoopType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_config() -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("line_width", 0.525)
        .float("outer_wall_line_width", 0.525)
        .float("inner_wall_line_width", 0.625)
        .float("initial_layer_line_width", 0.625)
        .float("layer_height", 0.1)
        .float("nozzle_diameter", 0.5)
        .build()
}

fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

/// (loop_type, width) for every width-carrying Outer/Inner wall point.
fn wall_widths(config: &ConfigView, layer_index: u32) -> Vec<(LoopType, f32)> {
    let module = ArachnePerimeters::from_config(config).unwrap();
    let regions = vec![make_region(10.0, 0.1 * (layer_index as f32 + 1.0))];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(layer_index, &regions, &paint, &mut output, config)
        .unwrap();

    let mut widths = Vec::new();
    for wall in output.wall_loops() {
        if !matches!(wall.loop_type, LoopType::Outer | LoopType::Inner) {
            continue;
        }
        for pt in &wall.path.points {
            if pt.width > 0.0 {
                widths.push((wall.loop_type.clone(), pt.width));
            }
        }
    }
    assert!(!widths.is_empty(), "expected wall points with widths");
    widths
}

#[test]
fn emitted_wall_widths_match_configured_values() {
    let config = make_config();
    let widths = wall_widths(&config, 5);

    let outer: Vec<f32> = widths
        .iter()
        .filter(|(loop_type, _)| *loop_type == LoopType::Outer)
        .map(|(_, w)| *w)
        .collect();
    let inner: Vec<f32> = widths
        .iter()
        .filter(|(loop_type, _)| *loop_type == LoopType::Inner)
        .map(|(_, w)| *w)
        .collect();

    // Every width-carrying point on the clean square's uniform runs must sit
    // at the configured width — the beading spacing (0.525 - 0.1*(1-pi/4) =
    // 0.5035) must never leak through un-converted.
    for w in &outer {
        assert!(
            (w - 0.525).abs() < 1e-4,
            "outer wall width must be the configured 0.525 mm, got {w:.4}"
        );
    }
    for w in &inner {
        assert!(
            (w - 0.625).abs() < 1e-4,
            "inner wall width must be the configured 0.625 mm, got {w:.4}"
        );
    }
}

#[test]
fn first_layer_emitted_width_uses_initial_layer_line_width() {
    let config = make_config();
    let widths = wall_widths(&config, 0);

    let outer: Vec<f32> = widths
        .iter()
        .filter(|(loop_type, _)| *loop_type == LoopType::Outer)
        .map(|(_, w)| *w)
        .collect();
    for w in &outer {
        assert!(
            (w - 0.625).abs() < 1e-4,
            "first-layer outer wall must use initial_layer_line_width 0.625 mm, got {w:.4}"
        );
    }
}
