#![allow(missing_docs)]

use slicer_core::algos::overhang_classifier::classify_layers;
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, FeedrateConfig, LayerCollectionIR, PrintEntity, RegionKey,
};

fn active_config() -> FeedrateConfig {
    FeedrateConfig {
        overhang_1_4_speed: 10.0,
        overhang_2_4_speed: 20.0,
        overhang_3_4_speed: 30.0,
        overhang_4_4_speed: 40.0,
        ..FeedrateConfig::default()
    }
}

fn make_point(x: f32, y: f32) -> slicer_ir::Point3WithWidth {
    slicer_ir::Point3WithWidth {
        x,
        y,
        z: 0.0,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn make_entity(role: ExtrusionRole, pts: Vec<slicer_ir::Point3WithWidth>) -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: pts,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "obj0".to_string(),
            region_id: 0,
        },
        topo_order: 0,
    }
}

fn empty_layer(global_layer_index: u32) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index,
        z: global_layer_index as f32 * 0.2,
        ..Default::default()
    }
}

fn layer_with_entity(
    global_layer_index: u32,
    role: ExtrusionRole,
    pts: Vec<slicer_ir::Point3WithWidth>,
) -> LayerCollectionIR {
    let mut l = empty_layer(global_layer_index);
    l.ordered_entities.push(make_entity(role, pts));
    l
}

fn square_wall_layer(idx: u32) -> LayerCollectionIR {
    let pts = vec![
        make_point(0.0, 0.0),
        make_point(10.0, 0.0),
        make_point(10.0, 10.0),
        make_point(0.0, 10.0),
    ];
    layer_with_entity(idx, ExtrusionRole::OuterWall, pts)
}

#[test]
fn short_circuit_on_zero_config() {
    let zero_config = FeedrateConfig {
        overhang_1_4_speed: 0.0,
        overhang_2_4_speed: 0.0,
        overhang_3_4_speed: 0.0,
        overhang_4_4_speed: 0.0,
        ..FeedrateConfig::default()
    };
    let mut layers = vec![
        square_wall_layer(0),
        layer_with_entity(
            1,
            ExtrusionRole::OuterWall,
            vec![make_point(5.0, 5.0), make_point(6.0, 6.0)],
        ),
    ];

    classify_layers(&mut layers, &zero_config);

    for layer in &layers {
        for entity in &layer.ordered_entities {
            for pt in &entity.path.points {
                assert_eq!(pt.overhang_quartile, None);
            }
        }
    }
}

#[test]
fn first_layer_all_none() {
    let mut layers = vec![layer_with_entity(
        0,
        ExtrusionRole::OuterWall,
        vec![make_point(5.0, 5.0)],
    )];

    classify_layers(&mut layers, &active_config());

    for pt in &layers[0].ordered_entities[0].path.points {
        assert_eq!(pt.overhang_quartile, None);
    }
}

#[test]
fn role_scope_guard() {
    let mut layers = vec![
        square_wall_layer(0),
        layer_with_entity(
            1,
            ExtrusionRole::SparseInfill,
            vec![make_point(5.0, 5.0), make_point(6.0, 6.0)],
        ),
    ];

    classify_layers(&mut layers, &active_config());

    for pt in &layers[1].ordered_entities[0].path.points {
        assert_eq!(pt.overhang_quartile, None);
    }
}

#[test]
fn quartile_boundary_inside() {
    let mut layers = vec![
        square_wall_layer(0),
        layer_with_entity(
            1,
            ExtrusionRole::OuterWall,
            vec![make_point(5.0, 5.0), make_point(5.0, 6.0)],
        ),
    ];

    classify_layers(&mut layers, &active_config());

    for pt in &layers[1].ordered_entities[0].path.points {
        assert_eq!(pt.overhang_quartile, Some(4));
    }
}

#[test]
fn quartile_boundary_q1() {
    let mut layers = vec![
        square_wall_layer(0),
        layer_with_entity(
            1,
            ExtrusionRole::OuterWall,
            vec![make_point(50.0, 50.0), make_point(51.0, 51.0)],
        ),
    ];

    classify_layers(&mut layers, &active_config());

    for pt in &layers[1].ordered_entities[0].path.points {
        assert_eq!(pt.overhang_quartile, Some(1));
    }
}
