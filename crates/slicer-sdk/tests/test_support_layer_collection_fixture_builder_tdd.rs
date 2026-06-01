//! TDD tests for [`LayerCollectionFixtureBuilder`].

use slicer_ir::{ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::test_prelude::*;

fn sample_point(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

#[test]
fn default_builder_produces_empty_layer() {
    let ir = LayerCollectionFixtureBuilder::new().build();
    assert_eq!(ir.global_layer_index, 0);
    assert!((ir.z - 0.0).abs() < f32::EPSILON);
    assert!(ir.ordered_entities.is_empty());
    assert!(ir.tool_changes.is_empty());
    assert!(ir.z_hops.is_empty());
    assert!(ir.annotations.is_empty());
    assert!(ir.retracts.is_empty());
    assert!(ir.travel_moves.is_empty());
}

#[test]
fn builder_threads_global_layer_index_and_z() {
    let ir = LayerCollectionFixtureBuilder::new()
        .global_layer_index(7)
        .z(1.4)
        .build();
    assert_eq!(ir.global_layer_index, 7);
    assert!((ir.z - 1.4).abs() < f32::EPSILON);
}

#[test]
fn builder_with_two_entities_and_one_tool_change_preserves_every_field() {
    let region_key = RegionKey {
        global_layer_index: 3,
        object_id: "obj-x".to_string(),
        region_id: 11,
    };
    let entity_a = print_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![sample_point(0.0, 0.0), sample_point(1.0, 0.0)],
        region_key.clone(),
        0,
    );
    let entity_b = print_entity(
        2,
        ExtrusionRole::InnerWall,
        vec![sample_point(0.0, 0.0), sample_point(0.0, 1.0)],
        region_key.clone(),
        1,
    );
    let tc = tool_change(1, 0, 2);

    let ir = LayerCollectionFixtureBuilder::new()
        .global_layer_index(3)
        .z(0.6)
        .add_entity(entity_a.clone())
        .add_entity(entity_b.clone())
        .add_tool_change(tc.clone())
        .build();

    assert_eq!(ir.global_layer_index, 3);
    assert!((ir.z - 0.6).abs() < f32::EPSILON);

    assert_eq!(ir.ordered_entities.len(), 2);
    assert_eq!(ir.ordered_entities[0], entity_a);
    assert_eq!(ir.ordered_entities[1], entity_b);

    assert_eq!(ir.tool_changes.len(), 1);
    assert_eq!(ir.tool_changes[0], tc);

    // Other fields untouched by the builder.
    assert!(ir.z_hops.is_empty());
    assert!(ir.annotations.is_empty());
    assert!(ir.retracts.is_empty());
    assert!(ir.travel_moves.is_empty());
}

#[test]
fn add_entity_preserves_insertion_order() {
    let ir = LayerCollectionFixtureBuilder::new()
        .add_entity(print_entity(
            10,
            ExtrusionRole::SparseInfill,
            Vec::new(),
            RegionKey::default(),
            0,
        ))
        .add_entity(print_entity(
            20,
            ExtrusionRole::TopSolidInfill,
            Vec::new(),
            RegionKey::default(),
            1,
        ))
        .build();
    assert_eq!(ir.ordered_entities[0].entity_id, 10);
    assert_eq!(ir.ordered_entities[1].entity_id, 20);
}

#[test]
fn add_tool_change_preserves_insertion_order() {
    let ir = LayerCollectionFixtureBuilder::new()
        .add_tool_change(tool_change(0, 0, 1))
        .add_tool_change(tool_change(3, 0, 2))
        .build();
    assert_eq!(ir.tool_changes.len(), 2);
    assert_eq!(ir.tool_changes[0].after_entity_index, 0);
    assert_eq!(ir.tool_changes[1].after_entity_index, 3);
}
