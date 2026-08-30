//! Packet 184 / D-164: canonical wall-width resolution in `classic-perimeters`.
//!
//! OrcaSlicer declares `outer_wall_line_width` / `inner_wall_line_width` as
//! `coFloatOrPercent` with `ratio_over = "nozzle_diameter"` and an upstream
//! default of `0` (see canonical `PrintConfigDef::init_fff_params` in
//! `PrintConfig.cpp`). A non-percent value `<= 0` is the *auto* sentinel:
//! canonical `Flow::new_from_config_width` routes it to
//! `Flow::auto_extrusion_width`, which returns `1.125 * nozzle_diameter` for
//! both `frExternalPerimeter` and `frPerimeter`.
//!
//! Test 1 locks the auto sentinel. Test 2 locks the packet-185 default move:
//! with the keys absent entirely, the canonical auto-0 default now applies
//! (`1.125 * nozzle_diameter`) — packet 185's AC-5 superseded packet 184's
//! "keep the legacy 0.4 mm fallback for the absent-key case" scope decision,
//! so absent keys behave identically to the explicit auto-0 sentinel in Test 1.

use classic_perimeters::ClassicPerimeters;
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

fn find_max_x(points: &[slicer_ir::Point3WithWidth]) -> f32 {
    points.iter().map(|p| p.x).fold(f32::MIN, f32::max)
}

#[test]
fn zero_width_resolves_to_canonical_auto_extrusion_width() {
    let nozzle_diameter = 0.6_f32;
    let expected = 1.125_f32 * nozzle_diameter; // 0.675 mm

    let config = ConfigViewBuilder::new()
        .int("wall_loops", 3)
        .float("nozzle_diameter", nozzle_diameter as f64)
        .float("outer_wall_line_width", 0.0)
        .float("layer_height", 0.2)
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    let outer: Vec<_> = walls.iter().filter(|w| w.perimeter_index == 0).collect();
    assert!(
        !outer.is_empty(),
        "Expected at least one outer (perimeter_index == 0) wall loop, got {} loops total",
        walls.len()
    );

    for wall in &outer {
        assert!(
            !wall.path.points.is_empty(),
            "Outer wall loop has no vertices"
        );
        for pt in &wall.path.points {
            assert!(
                (pt.width - expected).abs() < 0.005,
                "Outer wall vertex width {} != canonical auto_extrusion_width {} \
                 (1.125 * nozzle_diameter {})",
                pt.width,
                expected,
                nozzle_diameter
            );
        }
    }

    // Sanity: the outer centerline must be inset by half the resolved width.
    let outer_x = find_max_x(&outer[0].path.points);
    assert!(
        outer_x < 5.0,
        "Outer wall right edge X {outer_x} should be inset from the 5 mm contour"
    );
}

#[test]
fn absent_width_keys_resolve_to_canonical_auto_width() {
    // No outer_wall_line_width, no inner_wall_line_width, no line_width.
    // Packet 185 moved the defaults to canonical auto-0, so absent keys
    // resolve exactly like the explicit zero sentinel: 1.125 * nozzle_diameter.
    let nozzle_diameter = 0.6_f32;
    let expected = 1.125_f32 * nozzle_diameter; // 0.675 mm

    let config = ConfigViewBuilder::new()
        .int("wall_loops", 3)
        .float("nozzle_diameter", nozzle_diameter as f64)
        .float("layer_height", 0.2)
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "Expected at least one wall loop");

    let mut saw_outer = false;
    let mut saw_inner = false;
    for wall in walls {
        if wall.perimeter_index == 0 {
            saw_outer = true;
        } else {
            saw_inner = true;
        }
        for pt in &wall.path.points {
            assert!(
                (pt.width - expected).abs() < 0.005,
                "Wall (perimeter_index {}) vertex width {} != canonical auto \
                 width {} (1.125 * nozzle_diameter {})",
                wall.perimeter_index,
                pt.width,
                expected,
                nozzle_diameter
            );
        }
    }
    assert!(saw_outer, "Expected an outer (perimeter_index == 0) wall");
    assert!(saw_inner, "Expected at least one inner wall");
}
