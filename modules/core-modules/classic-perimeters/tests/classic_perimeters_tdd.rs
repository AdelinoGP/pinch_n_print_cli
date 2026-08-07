//! TDD tests for classic perimeter generation.
//!
//! Tests the ClassicPerimeters LayerModule implementation for the
//! Layer::Perimeters stage per docs/01_system_architecture.md.

use classic_perimeters::ClassicPerimeters;
use slicer_core::flow::line_width_to_spacing;
use slicer_ir::{ConfigView, ExPolygon, ExtrusionRole, LoopType, Polygon, WallBoundaryType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Create a square ExPolygon centered at origin with given side length in mm.
fn make_square(side_mm: f32) -> ExPolygon {
    square_polygon(0.0, 0.0, side_mm)
}

/// Create a config with specified wall_count and line_width.
fn make_config(wall_count: u32, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("line_width", line_width)
        .build()
}

/// Create a config with speed settings too.
fn make_speed_config(
    wall_count: u32,
    line_width: f64,
    outer_speed: f64,
    inner_speed: f64,
) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("line_width", line_width)
        .float("outer_wall_speed", outer_speed)
        .float("inner_wall_speed", inner_speed)
        .build()
}

/// Create a SliceRegionView with a single square polygon.
fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(make_square(side_mm))
        .build()
}

/// Audit-gap closure: a per-region `line_width` config reaches the emitted wall
/// geometry. Two runs with different `line_width` must produce proportionally
/// different outer-wall extrusion widths. Combined with
/// `region_mapping_applies_per_tool_config_overlay_to_painted_tool` (which proves
/// `tool_config:<n>:line_width` lands in a painted tool's `RegionPlan.config`),
/// this establishes per-tool `line_width` end-to-end: config → RegionPlan →
/// perimeter geometry.
#[test]
fn per_region_line_width_sets_emitted_wall_width() {
    let outer_width_for = |lw: f64| -> f32 {
        let config = make_config(2, lw);
        let module = ClassicPerimeters::from_config(&config).unwrap();
        let regions = vec![make_region(10.0, 0.2)];
        let paint = PaintRegionLayerView::new(0);
        let mut output = PerimeterOutputBuilder::new();
        module
            .run_perimeters(0, &regions, &paint, &mut output, &config)
            .unwrap();
        let outer = output
            .wall_loops()
            .iter()
            .find(|w| w.loop_type == LoopType::Outer)
            .expect("an outer wall loop must be emitted")
            .clone();
        outer.path.points[0].width
    };

    let w_narrow = outer_width_for(0.4);
    let w_wide = outer_width_for(0.8);

    assert!(
        (w_narrow - 0.4).abs() < 1e-4,
        "outer wall extrusion width must equal the per-region line_width 0.4; got {w_narrow}"
    );
    assert!(
        (w_wide - 0.8).abs() < 1e-4,
        "outer wall extrusion width must equal the per-region line_width 0.8; got {w_wide}"
    );
    assert!(
        w_wide > w_narrow,
        "a wider per-region line_width must yield a wider emitted wall ({w_wide} > {w_narrow})"
    );
}

#[test]
fn single_square_two_walls() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert_eq!(walls.len(), 2, "Expected 2 wall loops (outer + inner)");

    // Infill area should be non-empty and smaller than input
    let infill = output.infill_areas();
    assert!(!infill.is_empty(), "Infill areas should be computed");
}

#[test]
fn infill_boundary_inset_uses_flow_spacing_not_raw_width() {
    let line_width = 0.8_f32;
    let layer_height = 0.2_f32;
    let config = ConfigViewBuilder::new()
        .int("wall_count", 1)
        .float("line_width", line_width as f64)
        .float("outer_wall_line_width", line_width as f64)
        .float("inner_wall_line_width", line_width as f64)
        .float("layer_height", layer_height as f64)
        .build();
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let infill = output.infill_areas();
    assert_eq!(
        infill.len(),
        1,
        "the square should produce one infill polygon"
    );
    let contour = &infill[0][0].contour;
    let min_x = contour.points.iter().map(|p| p.x).min().unwrap() as f64;
    let max_x = contour.points.iter().map(|p| p.x).max().unwrap() as f64;
    let actual_side_mm = (max_x - min_x) / 10_000.0;
    let spacing = line_width_to_spacing(line_width, layer_height).unwrap() as f64;
    let expected_side_mm = 10.0 - line_width as f64 - 2.0 * spacing;

    assert!(
        (actual_side_mm - expected_side_mm).abs() < 0.01,
        "infill boundary must use flow spacing {spacing}, not raw width {line_width}: expected side {expected_side_mm}, got {actual_side_mm}"
    );
    assert!(
        (actual_side_mm - (10.0 - 3.0 * line_width as f64)).abs() > 0.03,
        "infill boundary must not use the raw line-width inset"
    );
}

#[test]
fn outer_wall_is_index_zero() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty());
    assert_eq!(walls[0].perimeter_index, 0, "Outer wall should be index 0");
    assert_eq!(
        walls[0].loop_type,
        LoopType::Outer,
        "First wall should be Outer"
    );
}

#[test]
fn inner_walls_correct_type() {
    let config = make_config(3, 0.4);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(walls.len() >= 3, "Expected at least 3 wall loops");

    for (i, wall) in walls.iter().enumerate().skip(1) {
        assert_eq!(
            wall.loop_type,
            LoopType::Inner,
            "Wall {} should be Inner",
            i
        );
        assert_eq!(
            wall.perimeter_index, i as u32,
            "Wall {} should have perimeter_index {}",
            i, i
        );
    }
}

#[test]
fn infill_area_computed() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let infill = output.infill_areas();
    assert!(!infill.is_empty(), "Infill areas should be non-empty");

    // Infill area should be smaller than original polygon
    // Original is 10x10=100mm^2, after 2 walls + half width inset, much smaller
    let infill_area: f64 = infill
        .iter()
        .flat_map(|call| call.iter())
        .map(|p| polygon_area_mm(&p.contour))
        .sum();
    assert!(
        infill_area < 100.0,
        "Infill area ({}) should be smaller than input (100mm^2)",
        infill_area
    );
    assert!(infill_area > 0.0, "Infill area should be positive");
}

#[test]
fn empty_polygon_no_output() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-1".to_string());
    region.set_region_id(1);
    region.set_polygons(Vec::new());
    region.set_infill_areas(Vec::new());
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    let regions = vec![region];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.wall_loops().len(),
        0,
        "No wall loops for empty input"
    );
    assert_eq!(output.infill_areas().len(), 0, "No infill for empty input");
}

#[test]
fn wall_count_zero() {
    let config = make_config(0, 0.4);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.wall_loops().len(),
        0,
        "No wall loops with wall_count=0"
    );
    // Infill areas should be the input polygons themselves
    assert!(
        !output.infill_areas().is_empty(),
        "Infill should be input polygons"
    );
}

/// DEV-125 — `alternate_extra_wall` adds exactly one wall on odd layers.
///
/// Canonical `process_classic` (`PerimeterGenerator.cpp`) does `loop_number++`
/// under `alternate_extra_wall && layer_id % 2 == 1 && !m_spiral_vase &&
/// sparse_infill_density > 0`; `loop_number` is 0-indexed so the wall count is
/// `loop_number + 1`. Before this fix the key was declared in the classic
/// manifest but read nowhere, so every layer emitted the base count.
#[test]
fn alternate_extra_wall_adds_one_wall_on_odd_layers() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("line_width", 0.4)
        .bool("alternate_extra_wall", true)
        .float("sparse_infill_density", 20.0)
        .build();
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);

    let run = |layer_index: u32| {
        let regions = vec![make_region(10.0, 0.2)];
        let mut output = PerimeterOutputBuilder::new();
        module
            .run_perimeters(layer_index, &regions, &paint, &mut output, &config)
            .unwrap();
        output.wall_loops().len()
    };

    // Layer 2 (even) is the baseline; layer 3 (odd) must carry one more loop.
    let even = run(2);
    let odd = run(3);
    assert_eq!(
        odd,
        even + 1,
        "alternate_extra_wall must add exactly one wall loop on odd layers \
         (even layer 2 emitted {even}, odd layer 3 emitted {odd})"
    );
}

/// DEV-125 — the two safety conjuncts must actually gate the bump. They are
/// only live because `spiral_vase` / `sparse_infill_density` are declared in
/// `classic-perimeters.toml`; an undeclared key is dropped by
/// `ConfigView::from_declared` and would silently read its fallback.
#[test]
fn alternate_extra_wall_suppressed_by_spiral_vase_and_zero_density() {
    let base = |spiral: bool, density: f64| {
        ConfigViewBuilder::new()
            .int("wall_count", 2)
            .float("line_width", 0.4)
            .bool("alternate_extra_wall", true)
            .bool("spiral_vase", spiral)
            .float("sparse_infill_density", density)
            .build()
    };

    let run = |config: &ConfigView| {
        let module = ClassicPerimeters::from_config(config).unwrap();
        let regions = vec![make_region(10.0, 0.2)];
        let paint = PaintRegionLayerView::new(0);
        let mut output = PerimeterOutputBuilder::new();
        module
            .run_perimeters(3, &regions, &paint, &mut output, config)
            .unwrap();
        output.wall_loops().len()
    };

    let enabled = run(&base(false, 20.0));
    assert_eq!(
        run(&base(true, 20.0)),
        enabled - 1,
        "spiral_vase must suppress the alternate_extra_wall bump"
    );
    assert_eq!(
        run(&base(false, 0.0)),
        enabled - 1,
        "sparse_infill_density == 0 must suppress the alternate_extra_wall bump"
    );
}

#[test]
fn seam_candidates_generated() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let seams = output.seam_candidates();
    assert!(
        !seams.is_empty(),
        "Seam candidates should be generated from outer wall corners"
    );
    // All seam candidates should have positive scores and correct Z
    for (pos, score) in seams {
        assert!(*score > 0.0, "Seam score should be positive, got {}", score);
        assert!((pos.z - 0.2).abs() < 0.01, "Seam Z should match layer Z");
    }
}

#[test]
fn speed_factor_from_config() {
    let config = make_speed_config(2, 0.4, 30.0, 60.0);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(walls.len() >= 2);

    // Outer wall: 30/50 = 0.6
    let outer = &walls[0];
    assert_eq!(outer.path.role, ExtrusionRole::OuterWall);
    assert!(
        (outer.path.speed_factor - 0.6).abs() < 0.01,
        "Outer speed_factor should be 30/50=0.6, got {}",
        outer.path.speed_factor
    );

    // Inner wall: 60/50 = 1.2
    let inner = &walls[1];
    assert_eq!(inner.path.role, ExtrusionRole::InnerWall);
    assert!(
        (inner.path.speed_factor - 1.2).abs() < 0.01,
        "Inner speed_factor should be 60/50=1.2, got {}",
        inner.path.speed_factor
    );

    // Verify boundary types
    assert_eq!(outer.boundary_type, WallBoundaryType::ExteriorSurface);
    assert_eq!(inner.boundary_type, WallBoundaryType::Interior);
}

fn make_overlap_config(infill_overlap: f64, top_bottom_overlap: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 1)
        .float("line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .float("layer_height", 0.2)
        .float("infill_wall_overlap", infill_overlap)
        .float("top_bottom_infill_wall_overlap", top_bottom_overlap)
        .build()
}

fn make_top_shell_region(side_mm: f32, z: f32, top_shell_index: Option<u8>) -> SliceRegionView {
    let mut region = make_region(side_mm, z);
    region.set_top_shell_index(top_shell_index);
    region
}

fn infill_area_mm2(output: &PerimeterOutputBuilder) -> f64 {
    output
        .infill_areas()
        .iter()
        .flat_map(|call| call.iter())
        .map(|polygon| polygon_area_mm(&polygon.contour))
        .sum()
}

#[test]
fn overlap_schema_declares_context_specific_percent_keys() {
    let manifest = include_str!("../classic-perimeters.toml");
    assert!(manifest.contains(
        "[config.schema.infill_wall_overlap]\ntype       = \"percent\"\ndefault    = \"15%\"\nratio_over = \"inner_wall_line_width\""
    ));
    assert!(manifest.contains(
        "[config.schema.top_bottom_infill_wall_overlap]\ntype       = \"percent\"\ndefault    = \"25%\"\nratio_over = \"inner_wall_line_width\""
    ));
}

#[test]
fn overlap_uses_top_bottom_key_on_layer_zero() {
    let config = make_overlap_config(0.0, 0.1);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);

    let mut layer_zero = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &regions, &paint, &mut layer_zero, &config)
        .unwrap();
    let mut regular_layer = PerimeterOutputBuilder::new();
    module
        .run_perimeters(1, &regions, &paint, &mut regular_layer, &config)
        .unwrap();

    assert!(
        infill_area_mm2(&layer_zero) > infill_area_mm2(&regular_layer),
        "layer zero must use top_bottom_infill_wall_overlap"
    );
}

#[test]
fn overlap_uses_top_bottom_key_for_topmost_top_shell() {
    let config = make_overlap_config(0.0, 0.1);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let topmost = vec![make_top_shell_region(10.0, 0.4, Some(0))];
    let non_topmost = vec![make_top_shell_region(10.0, 0.4, Some(1))];

    let mut topmost_output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(1, &topmost, &paint, &mut topmost_output, &config)
        .unwrap();
    let mut non_topmost_output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(1, &non_topmost, &paint, &mut non_topmost_output, &config)
        .unwrap();

    assert!(
        infill_area_mm2(&topmost_output) > infill_area_mm2(&non_topmost_output),
        "top_shell_index == Some(0) must use top_bottom_infill_wall_overlap"
    );
}

#[test]
fn only_one_wall_top_topmost_is_unconditional() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .float("layer_height", 0.2)
        .bool("only_one_wall_top", true)
        .float("min_width_top_surface", 5.0)
        .build();
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_top_shell_region(1.0, 0.4, Some(0))];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(1, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.wall_loops().len(),
        1,
        "topmost top sub-area must force exactly one wall despite min_width_top_surface"
    );
}

#[test]
fn only_one_wall_top_non_topmost_uses_min_width_top_surface() {
    let make_config_with_threshold = |threshold| {
        ConfigViewBuilder::new()
            .int("wall_count", 2)
            .float("line_width", 0.4)
            .float("outer_wall_line_width", 0.4)
            .float("inner_wall_line_width", 0.4)
            .float("layer_height", 0.2)
            .bool("only_one_wall_top", true)
            .float("min_width_top_surface", threshold)
            .build()
    };
    let mut region = make_top_shell_region(10.0, 0.4, Some(1));
    region.set_top_solid_fill(vec![make_square(4.0)]);
    let regions = vec![region];
    let paint = PaintRegionLayerView::new(0);

    let low_threshold = make_config_with_threshold(0.0);
    let low_module = ClassicPerimeters::from_config(&low_threshold).unwrap();
    let mut low_output = PerimeterOutputBuilder::new();
    low_module
        .run_perimeters(1, &regions, &paint, &mut low_output, &low_threshold)
        .unwrap();

    let high_threshold = make_config_with_threshold(5.0);
    let high_module = ClassicPerimeters::from_config(&high_threshold).unwrap();
    let mut high_output = PerimeterOutputBuilder::new();
    high_module
        .run_perimeters(1, &regions, &paint, &mut high_output, &high_threshold)
        .unwrap();

    let low_wall_count = low_output.wall_loops().len();
    let high_wall_count = high_output.wall_loops().len();
    assert!(
        high_wall_count > low_wall_count,
        "a non-topmost top sub-area below min_width_top_surface must keep full walls: \
         low={low_wall_count}, high={high_wall_count}"
    );
}

/// Helper: compute signed area of a polygon in mm^2 from scaled i64 coords.
fn polygon_area_mm(poly: &Polygon) -> f64 {
    let pts = &poly.points;
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut area: f64 = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += (pts[i].x as f64) * (pts[j].y as f64);
        area -= (pts[j].x as f64) * (pts[i].y as f64);
    }
    // Convert from scaled units^2 to mm^2
    (area.abs() / 2.0) / (10_000.0 * 10_000.0)
}
