//! TDD tests for the `slicer_sdk::test_support::fixtures` `*_base` fixture
//! bases (`print_entity_base`, `wall_loop_base`, `ordered_entity_view_base`)
//! and their FRU (Functionally-Redundant-Union) composition.

use slicer_ir::{
    ExtrusionRole, LoopType, Point3WithWidth, PrintEntity, RegionKey, WallBoundaryType, WallLoop,
};
use slicer_sdk::test_support::fixtures::{
    ordered_entity_view_base, print_entity_base, wall_loop_base,
};
use slicer_sdk::views::OrderedEntityView;

#[test]
fn print_entity_base_has_safe_fixture_values() {
    let entity = print_entity_base(ExtrusionRole::SparseInfill);

    assert_eq!(entity.entity_id, 0);
    assert_eq!(entity.path.points.len(), 1);
    assert_eq!(entity.path.role, ExtrusionRole::SparseInfill);
    assert_eq!(entity.role, ExtrusionRole::SparseInfill);
    assert_eq!(entity.path.speed_factor, 1.0);
    assert_eq!(entity.region_key, RegionKey::default());
    assert_eq!(entity.topo_order, 0);
    assert_eq!(entity.tool_index, 0);
}

#[test]
fn print_entity_base_composes_with_struct_update() {
    let entity = PrintEntity {
        topo_order: 7,
        ..print_entity_base(ExtrusionRole::SparseInfill)
    };

    assert_eq!(entity.topo_order, 7);
}

#[test]
fn wall_loop_base_preserves_fields_and_maps_roles() {
    let outer = wall_loop_base(LoopType::Outer, WallBoundaryType::ExteriorSurface);
    let inner = wall_loop_base(LoopType::Inner, WallBoundaryType::Interior);
    let thin = wall_loop_base(LoopType::ThinWall, WallBoundaryType::Interior);
    let other = wall_loop_base(LoopType::GapFill, WallBoundaryType::Interior);

    assert_eq!(outer.perimeter_index, 0);
    assert_eq!(outer.loop_type, LoopType::Outer);
    assert_eq!(outer.boundary_type, WallBoundaryType::ExteriorSurface);
    assert!(!outer.path.points.is_empty());
    assert_eq!(outer.width_profile.widths.len(), outer.path.points.len());
    assert!(outer.feature_flags.is_empty());
    assert_eq!(outer.path.role, ExtrusionRole::OuterWall);
    assert_eq!(inner.path.role, ExtrusionRole::InnerWall);
    assert_eq!(thin.path.role, ExtrusionRole::ThinWall);
    assert_eq!(other.path.role, ExtrusionRole::InnerWall);
}

#[test]
fn wall_loop_base_composes_with_struct_update() {
    let loop_fixture = WallLoop {
        perimeter_index: 3,
        ..wall_loop_base(LoopType::Inner, WallBoundaryType::Interior)
    };

    assert_eq!(loop_fixture.perimeter_index, 3);
}

#[test]
fn ordered_entity_view_base_has_safe_fixture_values() {
    let view = ordered_entity_view_base(ExtrusionRole::OuterWall);

    assert_eq!(view.original_index, 0);
    assert_eq!(view.tool_index, 0);
    assert_eq!(view.region_key, RegionKey::default());
    assert_eq!(view.role, ExtrusionRole::OuterWall);
    assert_eq!(view.start_point, Point3WithWidth::default());
    assert_eq!(view.end_point, Point3WithWidth::default());
    assert_eq!(view.point_count, 2);
}

#[test]
fn ordered_entity_view_base_composes_with_struct_update() {
    let view = OrderedEntityView {
        point_count: 5,
        ..ordered_entity_view_base(ExtrusionRole::OuterWall)
    };

    assert_eq!(view.point_count, 5);
}
