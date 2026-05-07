#![allow(missing_docs)]

use std::collections::HashMap;

use skirt_brim::SkirtBrim;
use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer,
};

// ---- Helpers ----

fn semver() -> SemVer {
    SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

fn make_entity_at(x: f32, y: f32, z: f32) -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![Point3WithWidth {
                x,
                y,
                z,
                width: 0.4,
                flow_factor: 1.0,
            }],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "obj1".to_string(),
            region_id: 1,
        },
        topo_order: 0,
    }
}

fn make_layer_with_entities(index: u32, z: f32, entities: Vec<PrintEntity>) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: index,
        z,
        ordered_entities: entities,
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    }
}

fn empty_config() -> ConfigView {
    ConfigView::from_map(HashMap::new())
}

fn custom_config(
    loops: u32,
    distance: f32,
    height: u32,
    brim: f32,
    line_w: f32,
    enabled: bool,
) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("skirt_brim_enabled".to_string(), ConfigValue::Bool(enabled));
    fields.insert("skirt_loops".to_string(), ConfigValue::Int(loops as i64));
    fields.insert(
        "skirt_distance".to_string(),
        ConfigValue::Float(distance as f64),
    );
    fields.insert("skirt_height".to_string(), ConfigValue::Int(height as i64));
    fields.insert("brim_width".to_string(), ConfigValue::Float(brim as f64));
    fields.insert("line_width".to_string(), ConfigValue::Float(line_w as f64));
    ConfigView::from_map(fields)
}

// ---- Tests ----

#[test]
fn from_config_defaults() {
    let sb = SkirtBrim::from_config(&empty_config()).unwrap();
    assert!(sb.enabled());
    assert_eq!(sb.skirt_loops(), 1);
    assert!((sb.skirt_distance() - 6.0).abs() < 0.001);
    assert_eq!(sb.skirt_height(), 1);
    assert!((sb.brim_width() - 0.0).abs() < 0.001);
    assert!((sb.line_width() - 0.4).abs() < 0.001);
}

#[test]
fn from_config_custom() {
    let cfg = custom_config(3, 10.0, 5, 4.0, 0.5, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    assert!(sb.enabled());
    assert_eq!(sb.skirt_loops(), 3);
    assert!((sb.skirt_distance() - 10.0).abs() < 0.001);
    assert_eq!(sb.skirt_height(), 5);
    assert!((sb.brim_width() - 4.0).abs() < 0.001);
    assert!((sb.line_width() - 0.5).abs() < 0.001);
}

#[test]
fn disabled_no_changes() {
    let cfg = custom_config(1, 6.0, 1, 0.0, 0.4, false);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(10.0, 10.0, 0.2)],
    )];
    let orig = layers.clone();
    sb.process(&mut layers).unwrap();
    assert_eq!(layers, orig);
}

#[test]
fn empty_layers_no_output() {
    let sb = SkirtBrim::from_config(&empty_config()).unwrap();
    let mut layers: Vec<LayerCollectionIR> = vec![];
    sb.process(&mut layers).unwrap();
    assert!(layers.is_empty());
}

#[test]
fn single_skirt_loop() {
    let cfg = custom_config(1, 6.0, 1, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(50.0, 50.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let skirt_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Skirt)
        .count();
    assert_eq!(skirt_count, 1, "expected 1 skirt entity, got {skirt_count}");
    // Skirt should be prepended (first entity)
    assert_eq!(layers[0].ordered_entities[0].role, ExtrusionRole::Skirt);
}

#[test]
fn skirt_has_correct_role() {
    let sb = SkirtBrim::from_config(&empty_config()).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(50.0, 50.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    for entity in &layers[0].ordered_entities {
        if entity.region_key.object_id == "__skirt__" {
            assert_eq!(entity.role, ExtrusionRole::Skirt);
            assert_eq!(entity.path.role, ExtrusionRole::Skirt);
        }
    }
}

#[test]
fn skirt_surrounds_print() {
    let cfg = custom_config(1, 6.0, 1, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![
            make_entity_at(10.0, 10.0, 0.2),
            make_entity_at(20.0, 20.0, 0.2),
        ],
    )];
    sb.process(&mut layers).unwrap();

    // entity bbox is [10, 10] to [20, 20]
    // skirt should be offset by 6.0mm, so skirt bbox should be [4, 4] to [26, 26]
    let skirt_entity = layers[0]
        .ordered_entities
        .iter()
        .find(|e| e.role == ExtrusionRole::Skirt)
        .expect("skirt entity should exist");

    let mut skirt_x_min = f32::MAX;
    let mut skirt_y_min = f32::MAX;
    let mut skirt_x_max = f32::MIN;
    let mut skirt_y_max = f32::MIN;

    for pt in &skirt_entity.path.points {
        skirt_x_min = skirt_x_min.min(pt.x);
        skirt_y_min = skirt_y_min.min(pt.y);
        skirt_x_max = skirt_x_max.max(pt.x);
        skirt_y_max = skirt_y_max.max(pt.y);
    }

    // Skirt should extend at least skirt_distance beyond entity bbox
    assert!(
        skirt_x_min <= 10.0 - 6.0 + 0.01,
        "skirt x_min={skirt_x_min} should be <= 4.0"
    );
    assert!(
        skirt_y_min <= 10.0 - 6.0 + 0.01,
        "skirt y_min={skirt_y_min} should be <= 4.0"
    );
    assert!(
        skirt_x_max >= 20.0 + 6.0 - 0.01,
        "skirt x_max={skirt_x_max} should be >= 26.0"
    );
    assert!(
        skirt_y_max >= 20.0 + 6.0 - 0.01,
        "skirt y_max={skirt_y_max} should be >= 26.0"
    );
}

#[test]
fn multiple_skirt_loops() {
    let cfg = custom_config(3, 6.0, 1, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(50.0, 50.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let skirt_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Skirt)
        .count();
    assert_eq!(
        skirt_count, 3,
        "expected 3 skirt entities, got {skirt_count}"
    );
}

#[test]
fn skirt_height_multiple_layers() {
    let cfg = custom_config(1, 6.0, 3, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![
        make_layer_with_entities(0, 0.2, vec![make_entity_at(50.0, 50.0, 0.2)]),
        make_layer_with_entities(1, 0.4, vec![make_entity_at(50.0, 50.0, 0.4)]),
        make_layer_with_entities(2, 0.6, vec![make_entity_at(50.0, 50.0, 0.6)]),
        make_layer_with_entities(3, 0.8, vec![make_entity_at(50.0, 50.0, 0.8)]),
    ];
    sb.process(&mut layers).unwrap();

    // First 3 layers should have skirt
    for (i, layer) in layers.iter().enumerate().take(3) {
        let has_skirt = layer
            .ordered_entities
            .iter()
            .any(|e| e.role == ExtrusionRole::Skirt);
        assert!(has_skirt, "layer {i} should have skirt entities");
    }

    // Layer 3 should NOT have skirt
    let has_skirt = layers[3]
        .ordered_entities
        .iter()
        .any(|e| e.role == ExtrusionRole::Skirt);
    assert!(!has_skirt, "layer 3 should NOT have skirt entities");
}

#[test]
fn brim_generates_paths() {
    let cfg = custom_config(0, 6.0, 1, 3.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![
        make_layer_with_entities(0, 0.2, vec![make_entity_at(50.0, 50.0, 0.2)]),
        make_layer_with_entities(1, 0.4, vec![make_entity_at(50.0, 50.0, 0.4)]),
    ];
    sb.process(&mut layers).unwrap();

    // Layer 0 should have brim entities
    let brim_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.region_key.object_id == "__brim__")
        .count();
    assert!(
        brim_count > 0,
        "layer 0 should have brim entities, got {brim_count}"
    );

    // Expected loops: ceil(3.0 / 0.4) = 8
    let expected_loops = (3.0_f32 / 0.4).ceil() as usize;
    assert_eq!(
        brim_count, expected_loops,
        "expected {expected_loops} brim loops, got {brim_count}"
    );

    // Layer 1 should NOT have brim entities
    let brim_on_layer1 = layers[1]
        .ordered_entities
        .iter()
        .any(|e| e.region_key.object_id == "__brim__");
    assert!(!brim_on_layer1, "layer 1 should NOT have brim entities");
}

#[test]
fn skirt_at_correct_z() {
    let cfg = custom_config(1, 6.0, 1, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(50.0, 50.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let skirt_entity = layers[0]
        .ordered_entities
        .iter()
        .find(|e| e.role == ExtrusionRole::Skirt)
        .expect("skirt entity should exist");

    for pt in &skirt_entity.path.points {
        assert!(
            (pt.z - 0.2).abs() < 0.001,
            "skirt point z={} should be 0.2",
            pt.z
        );
    }
}
