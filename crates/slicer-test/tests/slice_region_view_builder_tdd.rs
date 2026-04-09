//! TDD tests for SliceRegionViewBuilder producing SliceRegionView.

use slicer_ir::mm_to_units;
use slicer_test::fixtures::{square_polygon, SliceRegionViewBuilder};

#[test]
fn default_builder_produces_valid_view() {
    let view = SliceRegionViewBuilder::new().build();
    assert_eq!(view.object_id(), "obj-0");
    assert_eq!(*view.region_id(), 0);
    assert!((view.z() - 0.0).abs() < f32::EPSILON);
    assert!((view.effective_layer_height() - 0.2).abs() < f32::EPSILON);
    assert!(view.polygons().is_empty());
    assert!(view.infill_areas().is_empty());
    assert!(!view.has_nonplanar());
}

#[test]
fn builder_sets_object_id() {
    let view = SliceRegionViewBuilder::new()
        .object_id("test-obj")
        .build();
    assert_eq!(view.object_id(), "test-obj");
}

#[test]
fn builder_sets_region_id() {
    let view = SliceRegionViewBuilder::new()
        .region_id(42)
        .build();
    assert_eq!(*view.region_id(), 42);
}

#[test]
fn builder_sets_z() {
    let view = SliceRegionViewBuilder::new()
        .z(1.2)
        .build();
    assert!((view.z() - 1.2).abs() < f32::EPSILON);
}

#[test]
fn builder_sets_effective_layer_height() {
    let view = SliceRegionViewBuilder::new()
        .effective_layer_height(0.3)
        .build();
    assert!((view.effective_layer_height() - 0.3).abs() < f32::EPSILON);
}

#[test]
fn builder_add_polygon_adds_to_polygons() {
    let view = SliceRegionViewBuilder::new()
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .add_polygon(square_polygon(5.0, 5.0, 5.0))
        .build();
    assert_eq!(view.polygons().len(), 2);
}

#[test]
fn builder_add_infill_area_adds_independently() {
    let view = SliceRegionViewBuilder::new()
        .add_polygon(square_polygon(0.0, 0.0, 20.0))
        .add_infill_area(square_polygon(5.0, 5.0, 10.0))
        .build();
    assert_eq!(view.polygons().len(), 1);
    assert_eq!(view.infill_areas().len(), 1);
}

#[test]
fn builder_sets_has_nonplanar() {
    let view = SliceRegionViewBuilder::new()
        .has_nonplanar(true)
        .build();
    assert!(view.has_nonplanar());
}

#[test]
fn polygon_coordinates_use_scaled_units() {
    let view = SliceRegionViewBuilder::new()
        .add_polygon(square_polygon(0.0, 0.0, 2.0))
        .build();
    let pts = &view.polygons()[0].contour.points;
    assert_eq!(pts[0].x, mm_to_units(-1.0));
    assert_eq!(pts[0].y, mm_to_units(-1.0));
}

#[test]
fn infill_areas_default_to_polygons_when_none_added() {
    // When no explicit infill areas are added, they should clone from polygons
    // (matching the original behavior for backward compatibility)
    let view = SliceRegionViewBuilder::new()
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .build();
    assert_eq!(view.infill_areas().len(), 1);
}

#[test]
fn explicit_infill_areas_override_default() {
    // When infill areas are explicitly added, don't auto-clone from polygons
    let view = SliceRegionViewBuilder::new()
        .add_polygon(square_polygon(0.0, 0.0, 20.0))
        .add_polygon(square_polygon(5.0, 5.0, 5.0))
        .add_infill_area(square_polygon(2.0, 2.0, 8.0))
        .build();
    assert_eq!(view.polygons().len(), 2);
    assert_eq!(view.infill_areas().len(), 1);
}

#[test]
fn full_builder_chain_matches_doc_example() {
    let view = SliceRegionViewBuilder::new()
        .object_id("test-obj")
        .region_id(1)
        .z(1.2)
        .effective_layer_height(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 20.0))
        .add_polygon(square_polygon(5.0, 5.0, 10.0))
        .build();

    assert_eq!(view.object_id(), "test-obj");
    assert_eq!(*view.region_id(), 1);
    assert!((view.z() - 1.2).abs() < f32::EPSILON);
    assert!((view.effective_layer_height() - 0.2).abs() < f32::EPSILON);
    assert_eq!(view.polygons().len(), 2);
    // When no explicit infill areas, polygons are auto-cloned
    assert_eq!(view.infill_areas().len(), 2);
}
