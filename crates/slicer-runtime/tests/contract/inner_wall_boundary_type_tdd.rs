// inner_wall_boundary_type_tdd.rs — AC-2b contract test.
//
// AC-2b: When run_perimeters is invoked with a region whose segment_annotations
// carry two distinct Material tool indices on adjacent points, the INNER wall's
// boundary_type must be WallBoundaryType::MaterialBoundary{..} — NOT Interior.
//
// This verifies that inner walls flow through build_wall_flags (Step 3 wiring) and
// therefore inherit the same boundary-type derivation as outer walls.
// Both ClassicPerimeters and ArachnePerimeters are tested.

use std::collections::HashMap;

use arachne_perimeters::ArachnePerimeters;
use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ConfigView, PaintSemantic, PaintValue, WallBoundaryType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a config with wall_count=2, line_width=0.4.
fn config_2_walls() -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("line_width", 0.4)
        .build()
}

/// AC-2b: inner wall boundary_type is MaterialBoundary when adjacent vertices
/// carry different Material tool indices in segment_annotations.
///
/// Setup: 10 mm square polygon, 4 points.
/// Points 0,1 → ToolIndex(0); points 2,3 → ToolIndex(1).
/// This creates two material transitions (1→2 and 3→0), producing
/// WallBoundaryType::MaterialBoundary on the outer AND inner walls.
#[test]
fn inner_wall_has_material_boundary_with_multi_tool_region() {
    let config = config_2_walls();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    // 10 mm square polygon
    let poly = square_polygon(5.0, 5.0, 10.0);
    let num_points = poly.contour.points.len();
    assert!(
        num_points >= 4,
        "square_polygon must have at least 4 points, got {num_points}"
    );

    // Points 0,1 painted tool 0; points 2,3 painted tool 1 — two transitions.
    let mut paint_vals = vec![Some(PaintValue::ToolIndex(0)); num_points];
    paint_vals[2] = Some(PaintValue::ToolIndex(1));
    paint_vals[3] = Some(PaintValue::ToolIndex(1));
    let material_paint = vec![paint_vals];
    let mut segment_annotations = HashMap::new();
    segment_annotations.insert(PaintSemantic::Material, material_paint);

    let mut region = SliceRegionView::default();
    region.set_object_id("obj-ac2b".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![poly]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    region.set_segment_annotations(segment_annotations);
    // No bridge areas needed for this test.
    region.set_bridge_areas(vec![]);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not fail");

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "must emit at least one wall loop");

    // Confirm we have inner walls (perimeter_index > 0).
    let inner_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index > 0).collect();
    assert!(
        !inner_walls.is_empty(),
        "wall_count=2 must produce at least one inner wall (perimeter_index > 0)"
    );

    // Every inner wall must have WallBoundaryType::MaterialBoundary (NOT Interior).
    for wall in &inner_walls {
        assert!(
            matches!(
                wall.boundary_type,
                WallBoundaryType::MaterialBoundary { .. }
            ),
            "inner wall (perimeter_index={}) boundary_type must be MaterialBoundary{{..}}, \
             got {:?}",
            wall.perimeter_index,
            wall.boundary_type
        );
    }
}

/// AC-2b (Arachne): inner wall boundary_type is MaterialBoundary when adjacent vertices
/// carry different Material tool indices in segment_annotations.
///
/// Mirrors the ClassicPerimeters test above but runs through ArachnePerimeters to
/// confirm the same contract applies to both perimeter generators.
#[test]
fn arachne_inner_wall_has_material_boundary_with_multi_tool_region() {
    let config = config_2_walls();
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    // 10 mm square polygon
    let poly = square_polygon(5.0, 5.0, 10.0);
    let num_points = poly.contour.points.len();
    assert!(
        num_points >= 4,
        "square_polygon must have at least 4 points, got {num_points}"
    );

    // Points 0,1 painted tool 0; points 2,3 painted tool 1 — two transitions.
    let mut paint_vals = vec![Some(PaintValue::ToolIndex(0)); num_points];
    paint_vals[2] = Some(PaintValue::ToolIndex(1));
    paint_vals[3] = Some(PaintValue::ToolIndex(1));
    let material_paint = vec![paint_vals];
    let mut segment_annotations = HashMap::new();
    segment_annotations.insert(PaintSemantic::Material, material_paint);

    let mut region = SliceRegionView::default();
    region.set_object_id("obj-ac2b-arachne".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![poly]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    region.set_segment_annotations(segment_annotations);
    region.set_bridge_areas(vec![]);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not fail");

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "must emit at least one wall loop");

    // Confirm we have inner walls (perimeter_index > 0).
    let inner_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index > 0).collect();
    assert!(
        !inner_walls.is_empty(),
        "wall_count=2 must produce at least one inner wall (perimeter_index > 0)"
    );

    // Every inner wall must have WallBoundaryType::MaterialBoundary (NOT Interior).
    for wall in &inner_walls {
        assert!(
            matches!(
                wall.boundary_type,
                WallBoundaryType::MaterialBoundary { .. }
            ),
            "arachne inner wall (perimeter_index={}) boundary_type must be MaterialBoundary{{..}}, \
             got {:?}",
            wall.perimeter_index,
            wall.boundary_type
        );
    }
}
