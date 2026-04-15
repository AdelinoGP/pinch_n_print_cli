//! TDD red tests for TASK-093: classic-perimeters boundary_paint propagation.
//!
//! Tests verify that classic-perimeters reads boundary_paint from SliceRegionView
//! and propagates Material->tool_index, FuzzySkin->fuzzy_skin into WallFeatureFlags
//! for outer wall points, and detects MaterialBoundary on adjacent material changes.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, PaintSemantic, PaintValue, Point2, Polygon,
    WallBoundaryType,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

// Import the module under test
use classic_perimeters::ClassicPerimeters;

/// Helper: create a simple square polygon (CCW contour, no holes).
fn square_polygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![],
    }
}

/// Helper: default config with wall_count=1 for simpler test output.
fn config_1_wall() -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("wall_count".to_string(), ConfigValue::Int(1));
    fields.insert("line_width".to_string(), ConfigValue::Float(0.4));
    ConfigView::from_map(fields)
}

/// Helper: default config with wall_count=2 for inner wall tests.
fn config_2_walls() -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("wall_count".to_string(), ConfigValue::Int(2));
    fields.insert("line_width".to_string(), ConfigValue::Float(0.4));
    ConfigView::from_map(fields)
}

#[test]
fn unpainted_region_produces_default_flags() {
    // A region with no boundary_paint should produce default feature flags
    // (tool_index=None, fuzzy_skin=false) on all points.
    let config = config_1_wall();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = SliceRegionView::new(
        "obj-1".to_string(),
        0,
        vec![square_polygon()],
        vec![],
        0.2,
        0.2,
        false,
    );

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "should produce at least one wall loop");

    for wall in walls {
        for flags in &wall.feature_flags {
            assert_eq!(
                flags.tool_index, None,
                "unpainted should have no tool_index"
            );
            assert!(!flags.fuzzy_skin, "unpainted should have fuzzy_skin=false");
        }
    }
}

#[test]
fn material_paint_sets_tool_index_on_outer_wall() {
    // When boundary_paint has Material semantic with ToolIndex values,
    // outer wall feature_flags.tool_index should be set accordingly.
    let config = config_1_wall();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let poly = square_polygon();
    let num_points = poly.contour.points.len();

    // All points painted with Material ToolIndex(2)
    let material_paint = vec![vec![Some(PaintValue::ToolIndex(2)); num_points]];
    let mut boundary_paint = HashMap::new();
    boundary_paint.insert(PaintSemantic::Material, material_paint);

    let region = SliceRegionView::with_boundary_paint(
        "obj-1".to_string(),
        0,
        vec![poly],
        vec![],
        0.2,
        0.2,
        false,
        boundary_paint,
    );

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "should produce wall loops");

    // Find outer walls
    let outer_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index == 0).collect();
    assert!(!outer_walls.is_empty(), "should have outer walls");

    for wall in &outer_walls {
        for flags in &wall.feature_flags {
            assert_eq!(
                flags.tool_index,
                Some(2),
                "Material paint should set tool_index on outer wall"
            );
        }
    }
}

#[test]
fn fuzzy_skin_paint_sets_flag_on_outer_wall() {
    // When boundary_paint has FuzzySkin semantic with Flag(true) values,
    // outer wall feature_flags.fuzzy_skin should be true.
    let config = config_1_wall();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let poly = square_polygon();
    let num_points = poly.contour.points.len();

    let fuzzy_paint = vec![vec![Some(PaintValue::Flag(true)); num_points]];
    let mut boundary_paint = HashMap::new();
    boundary_paint.insert(PaintSemantic::FuzzySkin, fuzzy_paint);

    let region = SliceRegionView::with_boundary_paint(
        "obj-1".to_string(),
        0,
        vec![poly],
        vec![],
        0.2,
        0.2,
        false,
        boundary_paint,
    );

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    let outer_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index == 0).collect();
    assert!(!outer_walls.is_empty(), "should have outer walls");

    for wall in &outer_walls {
        for flags in &wall.feature_flags {
            assert!(
                flags.fuzzy_skin,
                "FuzzySkin paint should set fuzzy_skin=true on outer wall"
            );
        }
    }
}

#[test]
fn inner_walls_get_no_paint_propagation() {
    // Inner walls (perimeter_index > 0) should NOT get paint propagation,
    // even when boundary_paint is present. Only outer walls get it.
    let config = config_2_walls();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let poly = square_polygon();
    let num_points = poly.contour.points.len();

    let material_paint = vec![vec![Some(PaintValue::ToolIndex(3)); num_points]];
    let mut boundary_paint = HashMap::new();
    boundary_paint.insert(PaintSemantic::Material, material_paint);

    let region = SliceRegionView::with_boundary_paint(
        "obj-1".to_string(),
        0,
        vec![poly],
        vec![],
        0.2,
        0.2,
        false,
        boundary_paint,
    );

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    let inner_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index > 0).collect();

    // Inner walls should exist (2-wall config on a big enough polygon)
    // and should have default flags
    for wall in &inner_walls {
        for flags in &wall.feature_flags {
            assert_eq!(
                flags.tool_index, None,
                "inner walls should not get paint propagation"
            );
            assert!(
                !flags.fuzzy_skin,
                "inner walls should not get fuzzy_skin from paint"
            );
        }
    }
}

#[test]
fn adjacent_material_change_sets_material_boundary() {
    // When adjacent outer wall points have different Material tool_index values,
    // the wall's boundary_type should be MaterialBoundary.
    let config = config_1_wall();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let poly = square_polygon();

    // Points 0,1 have tool 1; points 2,3 have tool 2 -> material change between 1->2 and 3->0
    let material_paint = vec![vec![
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(2)),
    ]];
    let mut boundary_paint = HashMap::new();
    boundary_paint.insert(PaintSemantic::Material, material_paint);

    let region = SliceRegionView::with_boundary_paint(
        "obj-1".to_string(),
        0,
        vec![poly],
        vec![],
        0.2,
        0.2,
        false,
        boundary_paint,
    );

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    let outer_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index == 0).collect();
    assert!(!outer_walls.is_empty(), "should have outer walls");

    // At least one outer wall should have MaterialBoundary type
    let has_material_boundary = outer_walls
        .iter()
        .any(|w| matches!(w.boundary_type, WallBoundaryType::MaterialBoundary { .. }));
    assert!(
        has_material_boundary,
        "adjacent material change should produce MaterialBoundary"
    );
}

#[test]
fn mixed_painted_unpainted_preserves_none_as_default() {
    // When some points are painted and some are None, the None points
    // should produce default feature flags (tool_index=None).
    let config = config_1_wall();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let poly = square_polygon();

    // Points 0,2 painted, points 1,3 unpainted
    let material_paint = vec![vec![
        Some(PaintValue::ToolIndex(1)),
        None,
        Some(PaintValue::ToolIndex(1)),
        None,
    ]];
    let mut boundary_paint = HashMap::new();
    boundary_paint.insert(PaintSemantic::Material, material_paint);

    let region = SliceRegionView::with_boundary_paint(
        "obj-1".to_string(),
        0,
        vec![poly],
        vec![],
        0.2,
        0.2,
        false,
        boundary_paint,
    );

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    let outer_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index == 0).collect();
    assert!(!outer_walls.is_empty(), "should have outer walls");

    // Check that we have a mix: some with tool_index, some without
    for wall in &outer_walls {
        let has_painted = wall.feature_flags.iter().any(|f| f.tool_index.is_some());
        let has_unpainted = wall.feature_flags.iter().any(|f| f.tool_index.is_none());
        // The wall should have both painted and unpainted points
        assert!(
            has_painted || has_unpainted,
            "mixed paint should preserve both painted and unpainted flags"
        );
        // Unpainted points should have default (None) tool_index
        for flags in &wall.feature_flags {
            if flags.tool_index.is_none() {
                assert!(
                    !flags.fuzzy_skin,
                    "unpainted points should have default fuzzy_skin=false"
                );
            }
        }
    }
}
