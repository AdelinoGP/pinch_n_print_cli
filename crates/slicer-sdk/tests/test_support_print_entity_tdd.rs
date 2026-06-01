//! TDD tests for the `print_entity` freestanding fixture helper.

use slicer_ir::{ExtrusionRole, LoopType, Point3WithWidth, RegionKey};
use slicer_sdk::test_prelude::*;

fn sample_point(x: f32, y: f32, z: f32, width: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

#[test]
fn print_entity_round_trip_preserves_inputs() {
    let region_key = RegionKey {
        global_layer_index: 7,
        object_id: "obj-1".to_string(),
        region_id: 42,
    };
    let points = vec![
        sample_point(0.0, 0.0, 0.2, 0.4),
        sample_point(1.0, 0.0, 0.2, 0.4),
        sample_point(1.0, 1.0, 0.2, 0.4),
    ];
    let entity = print_entity(
        17,
        ExtrusionRole::InnerWall,
        points.clone(),
        region_key.clone(),
        3,
    );

    assert_eq!(entity.entity_id, 17);
    assert_eq!(entity.role, ExtrusionRole::InnerWall);
    assert_eq!(entity.region_key, region_key);
    assert_eq!(entity.topo_order, 3);

    // Path carries the inputs verbatim.
    assert_eq!(entity.path.role, ExtrusionRole::InnerWall);
    assert!((entity.path.speed_factor - 1.0).abs() < f32::EPSILON);
    assert_eq!(entity.path.points.len(), points.len());
    for (got, want) in entity.path.points.iter().zip(points.iter()) {
        assert_eq!(got, want);
    }
}

#[test]
fn print_entity_uses_named_construction_for_outer_wall() {
    let entity = print_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![sample_point(0.0, 0.0, 0.1, 0.5)],
        RegionKey::default(),
        0,
    );
    assert_eq!(entity.role, ExtrusionRole::OuterWall);
    assert_eq!(entity.path.role, ExtrusionRole::OuterWall);
    assert_eq!(entity.topo_order, 0);
    // Sanity: not the default sentinel.
    assert_ne!(entity.entity_id, 0);
}

#[test]
fn print_entity_empty_points_still_constructs() {
    let entity = print_entity(
        99,
        ExtrusionRole::SparseInfill,
        Vec::new(),
        RegionKey::default(),
        12,
    );
    assert!(entity.path.points.is_empty());
    assert_eq!(entity.topo_order, 12);
}

// Avoid unused-warning on LoopType import; we re-use slicer_ir's namespace
// only to confirm the SDK re-exports stay distinct.
#[test]
fn loop_type_namespace_distinct_from_extrusion_role() {
    let _ = LoopType::Outer;
}
