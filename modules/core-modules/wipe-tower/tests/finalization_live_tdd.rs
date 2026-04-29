#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer, ToolChange,
};
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};
use wipe_tower::WipeTower;

// ---- Helpers ----

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
        retracts: vec![],
        travel_moves: vec![],
    }
}

fn config_with(pairs: &[(&str, ConfigValue)]) -> ConfigView {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v.clone());
    }
    ConfigView::from_map(map)
}

fn wipe_tower_from(pairs: &[(&str, ConfigValue)]) -> WipeTower {
    WipeTower::from_config(&config_with(pairs)).expect("config should be valid")
}

fn tc(after: u32, from: u32, to: u32) -> ToolChange {
    ToolChange {
        after_entity_index: after,
        from_tool: from,
        to_tool: to,
    }
}

// ─── AC-1: run_finalization pushes wipe-tower entities for tool-change layers ─

#[test]
fn run_finalization_pushes_wipe_tower_entities_for_tool_change_layers() {
    let wt = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("wipe_tower_purge_volume", ConfigValue::Float(70.0)),
        ("wipe_tower_width", ConfigValue::Float(60.0)),
        ("line_width", ConfigValue::Float(0.4)),
    ]);

    let layer = make_layer(0, 0.2, vec![tc(0, 0, 1)]);
    let views = vec![LayerCollectionView::new(layer)];
    let config = config_with(&[]);
    let mut output = FinalizationOutputBuilder::new();
    wt.run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let pushes = output.entity_pushes();
    assert!(
        !pushes.is_empty(),
        "expected non-empty entity pushes for a tool-change layer"
    );
    for (layer_index, path, _region_key) in pushes {
        assert_eq!(*layer_index, 0, "all pushes must target layer index 0");
        assert_eq!(
            path.role,
            ExtrusionRole::WipeTower,
            "path role must be WipeTower"
        );
    }
}

// ─── AC-2: purge_volume controls entity-push count ───────────────────────────

#[test]
fn purge_volume_controls_finalization_push_count() {
    let wt_small = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("wipe_tower_purge_volume", ConfigValue::Float(70.0)),
        ("wipe_tower_width", ConfigValue::Float(60.0)),
        ("line_width", ConfigValue::Float(0.4)),
    ]);
    let wt_large = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("wipe_tower_purge_volume", ConfigValue::Float(140.0)),
        ("wipe_tower_width", ConfigValue::Float(60.0)),
        ("line_width", ConfigValue::Float(0.4)),
    ]);

    let layer_small = make_layer(0, 0.2, vec![tc(0, 0, 1)]);
    let layer_large = make_layer(0, 0.2, vec![tc(0, 0, 1)]);

    let views_small = vec![LayerCollectionView::new(layer_small)];
    let views_large = vec![LayerCollectionView::new(layer_large)];
    let config = config_with(&[]);

    let mut out_small = FinalizationOutputBuilder::new();
    wt_small
        .run_finalization(&views_small, &mut out_small, &config)
        .expect("run_finalization must succeed");

    let mut out_large = FinalizationOutputBuilder::new();
    wt_large
        .run_finalization(&views_large, &mut out_large, &config)
        .expect("run_finalization must succeed");

    let smaller_pushes = out_small.entity_pushes();
    let larger_pushes = out_large.entity_pushes();
    assert!(
        larger_pushes.len() > smaller_pushes.len(),
        "larger purge volume ({}) should produce more entity pushes than smaller ({})",
        larger_pushes.len(),
        smaller_pushes.len()
    );
}

// ─── AC-3: only layers with tool changes are targeted ────────────────────────

#[test]
fn run_finalization_targets_only_layers_with_tool_changes() {
    let wt = wipe_tower_from(&[("wipe_tower_enabled", ConfigValue::Bool(true))]);

    // layer 0: no tool changes; layer 1: one tool change
    let layer0 = make_layer(0, 0.2, vec![]);
    let layer1 = make_layer(1, 0.4, vec![tc(0, 0, 1)]);
    let views = vec![
        LayerCollectionView::new(layer0),
        LayerCollectionView::new(layer1),
    ];
    let config = config_with(&[]);
    let mut output = FinalizationOutputBuilder::new();
    wt.run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let pushes = output.entity_pushes();
    let targets: Vec<u32> = pushes.iter().map(|(li, _, _)| *li).collect();

    assert!(
        !targets.contains(&0),
        "layer 0 has no tool changes and must not be targeted"
    );
    assert!(
        targets.contains(&1),
        "layer 1 has a tool change and must be targeted"
    );
}

// ─── AC-Neg: disabled or no tool changes → zero pushes ───────────────────────

#[test]
fn disabled_or_no_tool_changes_emit_no_finalization_pushes() {
    let config = config_with(&[]);

    // Case A: wipe_tower_enabled=false with a layer that has a ToolChange
    let wt_disabled = wipe_tower_from(&[("wipe_tower_enabled", ConfigValue::Bool(false))]);
    let layer_with_tc = make_layer(0, 0.2, vec![tc(0, 0, 1)]);
    let views_a = vec![LayerCollectionView::new(layer_with_tc)];
    let mut out_a = FinalizationOutputBuilder::new();
    wt_disabled
        .run_finalization(&views_a, &mut out_a, &config)
        .expect("must not error when disabled");
    assert!(
        out_a.entity_pushes().is_empty(),
        "disabled wipe tower must emit no entity pushes"
    );

    // Case B: wipe_tower_enabled=true but layer has NO tool_changes
    let wt_enabled = wipe_tower_from(&[("wipe_tower_enabled", ConfigValue::Bool(true))]);
    let layer_no_tc = make_layer(0, 0.2, vec![]);
    let views_b = vec![LayerCollectionView::new(layer_no_tc)];
    let mut out_b = FinalizationOutputBuilder::new();
    wt_enabled
        .run_finalization(&views_b, &mut out_b, &config)
        .expect("must not error when no tool changes");
    assert!(
        out_b.entity_pushes().is_empty(),
        "enabled wipe tower with no tool changes must emit no entity pushes"
    );
}
