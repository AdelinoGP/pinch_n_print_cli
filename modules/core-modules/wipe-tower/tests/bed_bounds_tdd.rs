//! TDD scaffold: wipe-tower placement validation against config-supplied bed polygon.
//!
//! Packet 58_gcode-toolchange-purge-integration, Step 2 scaffolding.
//! AC6 — tower vertices are inside the bed polygon.
//!
//! Bed-containment only. The object-footprint non-intersection half of AC6 is
//! deferred to a follow-up packet (DEV-054 follow-up (i)) — the test name now
//! reflects this scope rather than over-claiming "outside_objects".

#![allow(missing_docs)]

use slicer_ir::{ConfigValue, ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::test_prelude::{
    print_entity, tool_change, ConfigViewBuilder, LayerCollectionFixtureBuilder,
};
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};
use wipe_tower::WipeTower;

/// Build a minimal ConfigView for the given key-value pairs.
fn config_from_pairs(pairs: &[(&str, ConfigValue)]) -> slicer_ir::ConfigView {
    let mut builder = ConfigViewBuilder::new();
    for (k, v) in pairs {
        match v {
            ConfigValue::Bool(b) => builder = builder.bool(*k, *b),
            ConfigValue::Float(f) => builder = builder.float(*k, *f),
            ConfigValue::Int(i) => builder = builder.int(*k, *i),
            ConfigValue::String(s) => builder = builder.string(*k, s.clone()),
            ConfigValue::List(l) => builder = builder.list(*k, l.clone()),
        }
    }
    builder.build()
}

/// Build a minimal single-layer IR with one ToolChange after entity 0.
fn layer_with_tool_change() -> slicer_ir::LayerCollectionIR {
    let entity = print_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![
            Point3WithWidth {
                x: 5.0,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: 6.0,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
        ],
        RegionKey {
            global_layer_index: 0,
            object_id: "cube".to_string(),
            region_id: 0,
        },
        0,
    );
    LayerCollectionFixtureBuilder::new()
        .global_layer_index(0)
        .z(0.2)
        .add_entity(entity)
        .add_tool_change(tool_change(0, 0, 1))
        .build()
}

/// AC6 — tower geometry within config-supplied bed polygon (bed-containment half).
///
/// Setup: `wipe_tower_enabled=true`, `bed_shape=[0,0, 250,0, 250,250, 0,250]`
/// (250×250 mm bed), `wipe_tower_x=10.0`, `wipe_tower_y=10.0`,
/// `wipe_tower_width=60.0`. When the wipe-tower module emits purge paths for
/// the first layer, `run_finalization` must return `Ok`, at least one
/// `WipeTower` entity must be produced, and all path points of every
/// `WipeTower` entity must lie within `[0, 250] × [0, 250]`.
///
/// The "outside object footprint" half of AC6 is deferred to a follow-up
/// packet (DEV-054 follow-up (i)) and intentionally NOT asserted here. The
/// test name reflects only the assertions that actually run.
#[test]
fn tower_geometry_within_config_bed_only() {
    let config = config_from_pairs(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("wipe_tower_x", ConfigValue::Float(10.0)),
        ("wipe_tower_y", ConfigValue::Float(10.0)),
        ("wipe_tower_width", ConfigValue::Float(60.0)),
        ("wipe_tower_purge_volume", ConfigValue::Float(70.0)),
        ("line_width", ConfigValue::Float(0.4)),
        ("retract_length", ConfigValue::Float(2.0)),
        (
            "bed_shape",
            ConfigValue::List(vec![
                ConfigValue::Float(0.0),
                ConfigValue::Float(0.0),
                ConfigValue::Float(250.0),
                ConfigValue::Float(0.0),
                ConfigValue::Float(250.0),
                ConfigValue::Float(250.0),
                ConfigValue::Float(0.0),
                ConfigValue::Float(250.0),
            ]),
        ),
    ]);

    let tower = WipeTower::from_config(&config).expect("from_config must succeed");

    let ir_layer = layer_with_tool_change();
    let mut layers = vec![ir_layer.clone()];
    let sdk_layers: Vec<LayerCollectionView> = vec![LayerCollectionView::new(ir_layer)];
    let mut output = FinalizationOutputBuilder::new();

    let result = tower.run_finalization(&sdk_layers, &mut output, &config);
    assert!(
        result.is_ok(),
        "AC6 FAIL: run_finalization returned Err for a valid tower inside bed polygon: {:?}",
        result.unwrap_err()
    );

    // Apply the insertions to get the final layer state.
    output
        .apply_to(&mut layers)
        .expect("apply_to must succeed for valid insertions");

    // Collect all WipeTower entities from the modified layer.
    let wipe_entities: Vec<&slicer_ir::PrintEntity> = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| matches!(e.role, ExtrusionRole::WipeTower))
        .collect();

    assert!(
        !wipe_entities.is_empty(),
        "AC6 FAIL: no WipeTower entities in layer after run_finalization"
    );

    // All path points of WipeTower entities must lie within [0, 250] × [0, 250].
    for entity in &wipe_entities {
        for pt in &entity.path.points {
            assert!(
                pt.x >= 0.0 && pt.x <= 250.0,
                "AC6 FAIL: WipeTower point X={:.3} lies outside [0, 250] bed bounds",
                pt.x
            );
            assert!(
                pt.y >= 0.0 && pt.y <= 250.0,
                "AC6 FAIL: WipeTower point Y={:.3} lies outside [0, 250] bed bounds",
                pt.y
            );
        }
    }

    // Object-footprint non-intersection is deferred to a follow-up packet
    // (DEV-054 follow-up (i)). When that lands, replace this comment with the
    // assertion and rename the test back to include "outside_objects".
}
