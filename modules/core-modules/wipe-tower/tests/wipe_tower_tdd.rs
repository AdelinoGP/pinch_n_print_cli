use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer, ToolChange,
};
use wipe_tower::WipeTower;

fn semver() -> SemVer {
    SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

fn dummy_entity(z: f32, index: u32) -> PrintEntity {
    PrintEntity {
        path: ExtrusionPath3D {
            points: vec![Point3WithWidth {
                x: 10.0,
                y: 10.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
            }],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            global_layer_index: index,
            object_id: "obj1".to_string(),
            region_id: 1,
        },
        topo_order: 0,
    }
}

fn make_layer(index: u32, z: f32, tool_changes: Vec<ToolChange>) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: index,
        z,
        ordered_entities: vec![dummy_entity(z, index)],
        tool_changes,
        z_hops: vec![],
        annotations: vec![],
    }
}

fn empty_config() -> ConfigView {
    ConfigView::from_map(HashMap::new(),)
}

fn enabled_config() -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("wipe_tower_enabled".to_string(), ConfigValue::Bool(true));
    ConfigView::from_map(fields)
}

fn custom_config(x: f32, y: f32, width: f32, purge_vol: f32, line_w: f32) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("wipe_tower_enabled".to_string(), ConfigValue::Bool(true));
    fields.insert("wipe_tower_x".to_string(), ConfigValue::Float(x as f64));
    fields.insert("wipe_tower_y".to_string(), ConfigValue::Float(y as f64));
    fields.insert(
        "wipe_tower_width".to_string(),
        ConfigValue::Float(width as f64),
    );
    fields.insert(
        "wipe_tower_purge_volume".to_string(),
        ConfigValue::Float(purge_vol as f64),
    );
    fields.insert("line_width".to_string(), ConfigValue::Float(line_w as f64));
    ConfigView::from_map(fields)
}

fn tc(after: u32, from: u32, to: u32) -> ToolChange {
    ToolChange {
        after_entity_index: after,
        from_tool: from,
        to_tool: to,
    }
}

// ---- Tests ----

#[test]
fn from_config_defaults() {
    let wt = WipeTower::from_config(&empty_config()).unwrap();
    assert!(!wt.enabled());
    assert!((wt.tower_x() - 0.0).abs() < 0.001);
    assert!((wt.tower_y() - 0.0).abs() < 0.001);
    assert!((wt.tower_width() - 60.0).abs() < 0.001);
    assert!((wt.purge_volume() - 70.0).abs() < 0.001);
}

#[test]
fn from_config_custom() {
    let cfg = custom_config(100.0, 200.0, 40.0, 140.0, 0.5);
    let wt = WipeTower::from_config(&cfg).unwrap();
    assert!(wt.enabled());
    assert!((wt.tower_x() - 100.0).abs() < 0.001);
    assert!((wt.tower_y() - 200.0).abs() < 0.001);
    assert!((wt.tower_width() - 40.0).abs() < 0.001);
    assert!((wt.purge_volume() - 140.0).abs() < 0.001);
    assert!((wt.line_width() - 0.5).abs() < 0.001);
}

#[test]
fn disabled_no_changes() {
    let wt = WipeTower::from_config(&empty_config()).unwrap();
    let mut layers = vec![make_layer(0, 0.2, vec![tc(0, 0, 1)])];
    let orig = layers.clone();
    wt.process(&mut layers).unwrap();
    assert_eq!(layers, orig);
}

#[test]
fn no_tool_changes_no_output() {
    let wt = WipeTower::from_config(&enabled_config()).unwrap();
    let mut layers = vec![make_layer(0, 0.2, vec![])];
    let orig = layers.clone();
    wt.process(&mut layers).unwrap();
    assert_eq!(layers, orig);
}

#[test]
fn single_tool_change_inserts_paths() {
    let wt = WipeTower::from_config(&enabled_config()).unwrap();
    let mut layers = vec![make_layer(0, 0.2, vec![tc(0, 0, 1)])];
    let entity_count_before = layers[0].ordered_entities.len();
    wt.process(&mut layers).unwrap();
    assert!(
        layers[0].ordered_entities.len() > entity_count_before,
        "entities should be added after tool change"
    );
}

#[test]
fn paths_have_wipe_tower_role() {
    let wt = WipeTower::from_config(&enabled_config()).unwrap();
    let mut layers = vec![make_layer(0, 0.2, vec![tc(0, 0, 1)])];
    let entity_count_before = layers[0].ordered_entities.len();
    wt.process(&mut layers).unwrap();

    for entity in &layers[0].ordered_entities[entity_count_before..] {
        assert_eq!(entity.role, ExtrusionRole::WipeTower);
        assert_eq!(entity.path.role, ExtrusionRole::WipeTower);
    }
}

#[test]
fn tower_at_configured_position() {
    let cfg = custom_config(100.0, 200.0, 60.0, 70.0, 0.4);
    let wt = WipeTower::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer(0, 0.2, vec![tc(0, 0, 1)])];
    let entity_count_before = layers[0].ordered_entities.len();
    wt.process(&mut layers).unwrap();

    for entity in &layers[0].ordered_entities[entity_count_before..] {
        for pt in &entity.path.points {
            assert!(
                pt.x >= 100.0 - 0.01 && pt.x <= 160.0 + 0.01,
                "x={} out of tower range [100, 160]",
                pt.x
            );
            assert!(pt.y >= 200.0 - 0.01, "y={} below tower_y=200", pt.y);
        }
    }
}

#[test]
fn purge_volume_affects_paths() {
    let cfg_small = custom_config(0.0, 0.0, 60.0, 70.0, 0.4);
    let cfg_large = custom_config(0.0, 0.0, 60.0, 140.0, 0.4);
    let wt_small = WipeTower::from_config(&cfg_small).unwrap();
    let wt_large = WipeTower::from_config(&cfg_large).unwrap();

    let mut layers_small = vec![make_layer(0, 0.2, vec![tc(0, 0, 1)])];
    let mut layers_large = vec![make_layer(0, 0.2, vec![tc(0, 0, 1)])];

    wt_small.process(&mut layers_small).unwrap();
    wt_large.process(&mut layers_large).unwrap();

    let count_small: usize = layers_small[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::WipeTower)
        .count();
    let count_large: usize = layers_large[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::WipeTower)
        .count();

    assert!(
        count_large > count_small,
        "larger purge volume ({count_large}) should produce more paths than smaller ({count_small})"
    );
}

#[test]
fn multiple_tool_changes_per_layer() {
    let wt = WipeTower::from_config(&enabled_config()).unwrap();
    let mut layers = vec![make_layer(0, 0.2, vec![tc(0, 0, 1), tc(0, 1, 2)])];
    let entity_count_before = layers[0].ordered_entities.len();
    wt.process(&mut layers).unwrap();

    let wipe_count: usize = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::WipeTower)
        .count();
    // Each tool change should produce at least one wipe entity
    assert!(
        wipe_count >= 2,
        "expected at least 2 wipe entities for 2 tool changes, got {wipe_count}"
    );
    assert!(layers[0].ordered_entities.len() > entity_count_before);
}

#[test]
fn multi_layer_tool_changes() {
    let wt = WipeTower::from_config(&enabled_config()).unwrap();
    let mut layers = vec![
        make_layer(0, 0.2, vec![tc(0, 0, 1)]),
        make_layer(1, 0.4, vec![tc(0, 1, 0)]),
    ];
    wt.process(&mut layers).unwrap();

    for (i, layer) in layers.iter().enumerate() {
        let has_wipe = layer
            .ordered_entities
            .iter()
            .any(|e| e.role == ExtrusionRole::WipeTower);
        assert!(has_wipe, "layer {i} should have wipe tower entities");
    }
}

#[test]
fn empty_layers_no_output() {
    let wt = WipeTower::from_config(&enabled_config()).unwrap();
    let mut layers: Vec<LayerCollectionIR> = vec![];
    wt.process(&mut layers).unwrap();
    assert!(layers.is_empty());
}
