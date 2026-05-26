//! TDD tests for top-surface-ironing rev 0.2 (Layer::Infill).
//!
//! After the slicing-promotion refactor, ironing runs as a Layer::Infill
//! module that reads polygon-precise top_solid_fill from SliceRegionView
//! (populated by PrePass::ShellClassification) and emits low-flow Ironing
//! paths via InfillOutputBuilder.
//!
//! Coordinate system: 1 unit = 100 nm; use `Point2::from_mm` for fixtures.

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{ConfigValue, ConfigView, ExPolygon, ExtrusionRole, Point2, Polygon};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;
use top_surface_ironing::TopSurfaceIroning;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn config_with(pairs: &[(&str, ConfigValue)]) -> ConfigView {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v.clone());
    }
    ConfigView::from_map(map)
}

fn default_config() -> ConfigView {
    config_with(&[
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_speed", ConfigValue::Float(20.0)),
        ("ironing_flow", ConfigValue::Float(0.10)),
        ("ironing_spacing_mm", ConfigValue::Float(0.1)),
        ("ironing_pattern", ConfigValue::String("rectilinear".to_string())),
    ])
}

/// 10mm × 10mm axis-aligned square centred at (0, 0).
fn square_polygon(side_mm: f32) -> ExPolygon {
    let half = side_mm / 2.0;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-half, -half),
                Point2::from_mm(half, -half),
                Point2::from_mm(half, half),
                Point2::from_mm(-half, half),
            ],
        },
        holes: vec![],
    }
}

/// L-shaped polygon, used to verify clip-to-polygon behaviour: a 10×10 square
/// with the upper-right 5×5 quadrant removed.
fn l_shape_polygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-5.0, -5.0),
                Point2::from_mm(5.0, -5.0),
                Point2::from_mm(5.0, 0.0),
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(0.0, 5.0),
                Point2::from_mm(-5.0, 5.0),
            ],
        },
        holes: vec![],
    }
}

/// Build a SliceRegionView with the given shell-index settings and an
/// optional `top_solid_fill` polygon list.
fn region_with(
    top_shell_index: Option<u8>,
    bottom_shell_index: Option<u8>,
    top_solid_fill: Vec<ExPolygon>,
) -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-test".to_string());
    region.set_region_id(0);
    region.set_z(1.0);
    region.set_effective_layer_height(0.2);
    region.set_top_shell_index(top_shell_index);
    region.set_bottom_shell_index(bottom_shell_index);
    region.set_top_solid_fill(top_solid_fill);
    region
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn topmost_layer_with_top_solid_fill_emits_ironing_paths() {
    let module = TopSurfaceIroning::on_print_start(&default_config()).unwrap();
    let region = region_with(Some(0), None, vec![square_polygon(10.0)]);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &default_config())
        .unwrap();

    let paths = output.ironing_paths();
    assert!(!paths.is_empty(), "expected at least one ironing path");
    for p in paths {
        assert_eq!(p.role, ExtrusionRole::Ironing);
        assert!(p.points.len() >= 2);
    }
}

#[test]
fn missing_top_shell_index_emits_no_ironing() {
    let module = TopSurfaceIroning::on_print_start(&default_config()).unwrap();
    let region = region_with(None, None, vec![]);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &default_config())
        .unwrap();

    assert!(
        output.ironing_paths().is_empty(),
        "regions with top_shell_index=None must not emit ironing"
    );
}

#[test]
fn interior_top_shell_layers_emit_no_ironing() {
    // top_shell_index = Some(1) means the region is 1 layer below the exposed
    // top — only Some(0) (the actual topmost exposed surface) gets ironed.
    let module = TopSurfaceIroning::on_print_start(&default_config()).unwrap();
    let region = region_with(Some(1), None, vec![square_polygon(10.0)]);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &default_config())
        .unwrap();

    assert!(
        output.ironing_paths().is_empty(),
        "depth-1 top-shell layer must not be ironed"
    );
}

#[test]
fn disabled_config_emits_no_ironing() {
    let cfg = config_with(&[
        ("ironing_enabled", ConfigValue::Bool(false)),
        ("ironing_speed", ConfigValue::Float(20.0)),
        ("ironing_flow", ConfigValue::Float(0.10)),
        ("ironing_spacing_mm", ConfigValue::Float(0.1)),
        ("ironing_pattern", ConfigValue::String("rectilinear".to_string())),
    ]);
    let module = TopSurfaceIroning::on_print_start(&cfg).unwrap();
    let region = region_with(Some(0), None, vec![square_polygon(10.0)]);
    let mut output = InfillOutputBuilder::new();

    module.run_infill(0, &[region], &mut output, &cfg).unwrap();

    assert!(output.ironing_paths().is_empty());
}

#[test]
fn spacing_governs_stroke_count_lower_bound() {
    // 10mm × 10mm square at 0.1mm spacing → ≥ 80 vertices (40+ strokes ×
    // 2 endpoints). Loose lower bound — clip-to-polygon trimming reduces the
    // exact count but the square is convex so every row should clip cleanly.
    let module = TopSurfaceIroning::on_print_start(&default_config()).unwrap();
    let region = region_with(Some(0), None, vec![square_polygon(10.0)]);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &default_config())
        .unwrap();

    let total_points: usize = output.ironing_paths().iter().map(|p| p.points.len()).sum();
    assert!(
        total_points >= 80,
        "expected ≥ 80 ironing points for 10mm² @ 0.1mm spacing, got {total_points}"
    );
}

#[test]
fn bottom_only_region_emits_no_ironing() {
    // bottom_shell_index=Some(0) (exposed bottom) — ironing is a top-surface
    // feature; bottom exposure must not trigger it.
    let module = TopSurfaceIroning::on_print_start(&default_config()).unwrap();
    let region = region_with(None, Some(0), vec![]);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &default_config())
        .unwrap();

    assert!(output.ironing_paths().is_empty());
}

#[test]
fn zero_flow_config_rejected_at_on_print_start() {
    let cfg = config_with(&[
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_speed", ConfigValue::Float(20.0)),
        ("ironing_flow", ConfigValue::Float(0.0)),
        ("ironing_spacing_mm", ConfigValue::Float(0.1)),
        ("ironing_pattern", ConfigValue::String("rectilinear".to_string())),
    ]);
    let err = TopSurfaceIroning::on_print_start(&cfg)
        .expect_err("ironing_flow = 0.0 must be rejected");
    let msg = err.message.to_string();
    assert!(
        msg.contains("ironing_flow"),
        "error message must mention ironing_flow, got '{msg}'"
    );
}

#[test]
fn unsupported_pattern_rejected_at_on_print_start() {
    let cfg = config_with(&[
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_speed", ConfigValue::Float(20.0)),
        ("ironing_flow", ConfigValue::Float(0.10)),
        ("ironing_spacing_mm", ConfigValue::Float(0.1)),
        ("ironing_pattern", ConfigValue::String("concentric".to_string())),
    ]);
    let err = TopSurfaceIroning::on_print_start(&cfg)
        .expect_err("non-rectilinear pattern must be rejected");
    let msg = err.message.to_string();
    assert!(
        msg.contains("ironing_pattern"),
        "error message must mention ironing_pattern, got '{msg}'"
    );
}

#[test]
fn l_shape_clip_keeps_strokes_inside_concave_polygon() {
    // L-shape: lower 10×5 strip plus left 5×5 column. Strokes whose midpoints
    // fall in the cut-out 5×5 quadrant must not be emitted (or must be clipped
    // away). We verify by computing the bounding-box-centred midpoint of every
    // stroke and confirming it lies inside the L-shape polygon.
    let module = TopSurfaceIroning::on_print_start(&default_config()).unwrap();
    let region = region_with(Some(0), None, vec![l_shape_polygon()]);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &default_config())
        .unwrap();

    let paths = output.ironing_paths();
    assert!(!paths.is_empty(), "L-shape must still produce some strokes");
    // Stroke endpoints come in (start, end) pairs. For each pair, midpoint
    // X must lie within the L-shape's solid mass, i.e. not in the upper-right
    // cut-out (x > 0 && y > 0).
    for path in paths {
        for pair in path.points.chunks_exact(2) {
            let midx = (pair[0].x + pair[1].x) / 2.0;
            let midy = (pair[0].y + pair[1].y) / 2.0;
            assert!(
                !(midx > 0.0 && midy > 0.0),
                "stroke midpoint ({midx:.2}, {midy:.2}) leaked into the L-shape cut-out"
            );
        }
    }
}

#[test]
fn cross_region_isolation_does_not_emit_for_uncovered_regions() {
    // Two regions on the same layer; only region A has top_shell_index=Some(0).
    // Region B (top_shell_index=None) must not contribute any ironing path.
    let module = TopSurfaceIroning::on_print_start(&default_config()).unwrap();
    let region_a = region_with(Some(0), None, vec![square_polygon(10.0)]);
    let region_b = region_with(None, None, vec![]);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region_a, region_b], &mut output, &default_config())
        .unwrap();

    // The output is region-agnostic at this layer, but the stroke count must
    // match the single-region case (smoke check that B did not contribute).
    let total_points: usize = output.ironing_paths().iter().map(|p| p.points.len()).sum();
    let mut a_only_output = InfillOutputBuilder::new();
    let region_a_only = region_with(Some(0), None, vec![square_polygon(10.0)]);
    module
        .run_infill(0, &[region_a_only], &mut a_only_output, &default_config())
        .unwrap();
    let expected: usize = a_only_output
        .ironing_paths()
        .iter()
        .map(|p| p.points.len())
        .sum();
    assert_eq!(total_points, expected);
}
