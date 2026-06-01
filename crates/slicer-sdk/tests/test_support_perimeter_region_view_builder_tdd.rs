//! TDD tests for PerimeterRegionViewBuilder producing PerimeterRegionView.

use slicer_ir::{ExtrusionRole, LoopType, WallBoundaryType, WallLoop, WidthProfile};
use slicer_sdk::test_support::fixtures::{rect_path, square_polygon, PerimeterRegionViewBuilder};

#[test]
fn default_builder_produces_valid_view() {
    let view = PerimeterRegionViewBuilder::new().build();
    assert_eq!(view.object_id(), "obj-0");
    assert_eq!(*view.region_id(), 0);
    assert!(view.wall_loops().is_empty());
    assert!(view.infill_areas().is_empty());
}

#[test]
fn builder_sets_object_id() {
    let view = PerimeterRegionViewBuilder::new()
        .object_id("test-obj")
        .build();
    assert_eq!(view.object_id(), "test-obj");
}

#[test]
fn builder_sets_region_id() {
    let view = PerimeterRegionViewBuilder::new().region_id(42).build();
    assert_eq!(*view.region_id(), 42);
}

#[test]
fn add_outer_wall_sets_loop_type_outer() {
    let path = rect_path(0.0, 0.0, 10.0, 0.4);
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall(path)
        .build();
    assert_eq!(view.wall_loops().len(), 1);
    assert_eq!(view.wall_loops()[0].loop_type, LoopType::Outer);
}

#[test]
fn add_outer_wall_sets_perimeter_index_zero() {
    let path = rect_path(0.0, 0.0, 10.0, 0.4);
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall(path)
        .build();
    assert_eq!(view.wall_loops()[0].perimeter_index, 0);
}

#[test]
fn add_inner_wall_sets_loop_type_inner() {
    let path = rect_path(0.0, 0.0, 8.0, 0.4);
    let view = PerimeterRegionViewBuilder::new()
        .add_inner_wall(path)
        .build();
    assert_eq!(view.wall_loops().len(), 1);
    assert_eq!(view.wall_loops()[0].loop_type, LoopType::Inner);
}

#[test]
fn inner_walls_auto_increment_perimeter_index() {
    let view = PerimeterRegionViewBuilder::new()
        .add_inner_wall(rect_path(0.0, 0.0, 8.0, 0.4))
        .add_inner_wall(rect_path(0.0, 0.0, 6.0, 0.4))
        .add_inner_wall(rect_path(0.0, 0.0, 4.0, 0.4))
        .build();
    assert_eq!(view.wall_loops().len(), 3);
    assert_eq!(view.wall_loops()[0].perimeter_index, 1);
    assert_eq!(view.wall_loops()[1].perimeter_index, 2);
    assert_eq!(view.wall_loops()[2].perimeter_index, 3);
}

#[test]
fn add_wall_loop_preserves_custom_wall() {
    let custom_loop = WallLoop {
        perimeter_index: 5,
        loop_type: LoopType::ThinWall,
        path: rect_path(0.0, 0.0, 4.0, 0.3),
        width_profile: WidthProfile {
            widths: vec![0.3; 4],
        },
        feature_flags: vec![],
        boundary_type: WallBoundaryType::ExteriorSurface,
    };
    let view = PerimeterRegionViewBuilder::new()
        .add_wall_loop(custom_loop)
        .build();
    assert_eq!(view.wall_loops().len(), 1);
    assert_eq!(view.wall_loops()[0].loop_type, LoopType::ThinWall);
    assert_eq!(view.wall_loops()[0].perimeter_index, 5);
    assert_eq!(
        view.wall_loops()[0].boundary_type,
        WallBoundaryType::ExteriorSurface
    );
}

#[test]
fn add_infill_area_adds_to_view() {
    let view = PerimeterRegionViewBuilder::new()
        .add_infill_area(square_polygon(0.0, 0.0, 10.0))
        .add_infill_area(square_polygon(5.0, 5.0, 5.0))
        .build();
    assert_eq!(view.infill_areas().len(), 2);
}

#[test]
fn default_boundary_type_is_interior() {
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall(rect_path(0.0, 0.0, 10.0, 0.4))
        .build();
    assert_eq!(
        view.wall_loops()[0].boundary_type,
        WallBoundaryType::Interior
    );
}

#[test]
fn default_feature_flags_are_empty() {
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall(rect_path(0.0, 0.0, 10.0, 0.4))
        .build();
    assert!(view.wall_loops()[0].feature_flags.is_empty());
}

#[test]
fn default_width_profile_matches_path_width() {
    let width = 0.45;
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall(rect_path(0.0, 0.0, 10.0, width))
        .build();
    let wl = &view.wall_loops()[0];
    assert_eq!(wl.width_profile.widths.len(), wl.path.points.len());
    for w in &wl.width_profile.widths {
        assert!((*w - width).abs() < f32::EPSILON);
    }
}

#[test]
fn rect_path_produces_rectangle() {
    let path = rect_path(0.0, 0.0, 10.0, 0.4);
    assert_eq!(path.points.len(), 4);
    assert_eq!(path.role, ExtrusionRole::OuterWall);
    assert!((path.speed_factor - 1.0).abs() < f32::EPSILON);
}

#[test]
fn rect_path_points_have_correct_width() {
    let width = 0.5;
    let path = rect_path(0.0, 0.0, 10.0, width);
    for pt in &path.points {
        assert!((pt.width - width).abs() < f32::EPSILON);
    }
}

#[test]
fn rect_path_points_are_at_z_zero() {
    let path = rect_path(5.0, 5.0, 10.0, 0.4);
    for pt in &path.points {
        assert!((pt.z - 0.0).abs() < f32::EPSILON);
    }
}

#[test]
fn rect_path_forms_closed_rectangle_shape() {
    let path = rect_path(1.0, 2.0, 6.0, 0.4);
    // Should form a rectangle centered at (1, 2) with side 6
    let half = 3.0_f32;
    let cx = 1.0_f32;
    let cy = 2.0_f32;

    // Check corners: (cx-half, cy-half), (cx+half, cy-half), (cx+half, cy+half), (cx-half, cy+half)
    let expected = [
        (cx - half, cy - half),
        (cx + half, cy - half),
        (cx + half, cy + half),
        (cx - half, cy + half),
    ];
    assert_eq!(path.points.len(), 4);
    for (pt, (ex, ey)) in path.points.iter().zip(expected.iter()) {
        assert!((pt.x - ex).abs() < f32::EPSILON, "x: {} != {}", pt.x, ex);
        assert!((pt.y - ey).abs() < f32::EPSILON, "y: {} != {}", pt.y, ey);
    }
}

#[test]
fn full_builder_chain() {
    let view = PerimeterRegionViewBuilder::new()
        .object_id("perim-obj")
        .region_id(7)
        .add_outer_wall(rect_path(0.0, 0.0, 20.0, 0.45))
        .add_inner_wall(rect_path(0.0, 0.0, 18.0, 0.4))
        .add_inner_wall(rect_path(0.0, 0.0, 16.0, 0.4))
        .add_infill_area(square_polygon(0.0, 0.0, 14.0))
        .build();

    assert_eq!(view.object_id(), "perim-obj");
    assert_eq!(*view.region_id(), 7);
    assert_eq!(view.wall_loops().len(), 3);
    assert_eq!(view.wall_loops()[0].loop_type, LoopType::Outer);
    assert_eq!(view.wall_loops()[0].perimeter_index, 0);
    assert_eq!(view.wall_loops()[1].loop_type, LoopType::Inner);
    assert_eq!(view.wall_loops()[1].perimeter_index, 1);
    assert_eq!(view.wall_loops()[2].loop_type, LoopType::Inner);
    assert_eq!(view.wall_loops()[2].perimeter_index, 2);
    assert_eq!(view.infill_areas().len(), 1);
}
