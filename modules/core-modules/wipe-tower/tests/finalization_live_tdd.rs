#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, ToolChange,
};
use slicer_sdk::test_prelude::{print_entity, tool_change, LayerCollectionFixtureBuilder};
use slicer_sdk::traits::{
    FinalizationModule, FinalizationOutputBuilder, LayerCollectionView, MergeOp,
};
use wipe_tower::WipeTower;

/// Standard 250×250 bed-shape ConfigValue used by every test below.
///
/// Packet 58 made `bed_shape` a config key consulted by `run_finalization` for
/// tower-corner containment. Tests that omit it fall back to the constructor's
/// default polygon, but supplying it explicitly keeps the fixture honest and
/// guards against future tightening of the fallback path.
fn bed_shape_250() -> ConfigValue {
    ConfigValue::List(vec![
        ConfigValue::Float(0.0),
        ConfigValue::Float(0.0),
        ConfigValue::Float(250.0),
        ConfigValue::Float(0.0),
        ConfigValue::Float(250.0),
        ConfigValue::Float(250.0),
        ConfigValue::Float(0.0),
        ConfigValue::Float(250.0),
    ])
}

/// Count insert-entity records targeting `layer_index` with role `WipeTower`.
///
/// Packet 58 migrated wipe-tower from `push_entity_with_priority` (which feeds
/// the legacy `entity_pushes()` slice) to `insert_entity_at` (which records
/// `MergeOp::InsertEntityAt` instead). These tests count the new records.
fn wipe_tower_inserts_for_layer(
    output: &FinalizationOutputBuilder,
    layer_index: u32,
) -> Vec<(u32, ExtrusionPath3D, RegionKey)> {
    output
        .merge_ops()
        .filter_map(|op| match op {
            MergeOp::InsertEntityAt {
                layer,
                path,
                region_key,
                ..
            } if *layer == layer_index && matches!(path.role, ExtrusionRole::WipeTower) => {
                Some((*layer, path.clone(), region_key.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Count wipe-tower inserts across every layer.
fn wipe_tower_inserts_total(output: &FinalizationOutputBuilder) -> Vec<u32> {
    output
        .merge_ops()
        .filter_map(|op| match op {
            MergeOp::InsertEntityAt { layer, path, .. }
                if matches!(path.role, ExtrusionRole::WipeTower) =>
            {
                Some(*layer)
            }
            _ => None,
        })
        .collect()
}

// ---- Helpers ----

fn dummy_entity(z: f32, index: u32) -> PrintEntity {
    print_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![Point3WithWidth {
            x: 10.0,
            y: 10.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        }],
        RegionKey {
            global_layer_index: index,
            object_id: "obj1".to_string(),
            region_id: 1,
            variant_chain: Vec::new(),
        },
        0,
    )
}

fn make_layer(index: u32, z: f32, tool_changes: Vec<ToolChange>) -> LayerCollectionIR {
    tool_changes
        .into_iter()
        .fold(
            LayerCollectionFixtureBuilder::new()
                .global_layer_index(index)
                .z(z)
                .add_entity(dummy_entity(z, index)),
            |b, tc| b.add_tool_change(tc),
        )
        .build()
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

// ─── AC-1: run_finalization pushes wipe-tower entities for tool-change layers ─

#[test]
fn run_finalization_pushes_wipe_tower_entities_for_tool_change_layers() {
    let wt = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("wipe_tower_purge_volume", ConfigValue::Float(70.0)),
        ("wipe_tower_width", ConfigValue::Float(60.0)),
        ("line_width", ConfigValue::Float(0.4)),
        ("bed_shape", bed_shape_250()),
    ]);

    let layer = make_layer(0, 0.2, vec![tool_change(0, 0, 1)]);
    let views = vec![LayerCollectionView::new(layer)];
    let config = config_with(&[("bed_shape", bed_shape_250())]);
    let mut output = FinalizationOutputBuilder::new();
    wt.run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let inserts = wipe_tower_inserts_for_layer(&output, 0);
    assert!(
        !inserts.is_empty(),
        "expected non-empty wipe-tower inserts for a tool-change layer"
    );
    for (layer_index, path, _region_key) in inserts {
        assert_eq!(layer_index, 0, "all inserts must target layer index 0");
        assert_eq!(
            path.role,
            ExtrusionRole::WipeTower,
            "path role must be WipeTower"
        );
    }
}

// ─── Regression: purge entities carry the destination tool (tc.to_tool) ──────

/// The wipe tower flushes the OLD filament by extruding the INCOMING one, so a
/// purge inserted for a tool change `from_tool → to_tool` must carry
/// `tool_index == to_tool` — NOT 0, and NOT the region_id (D-125 invariant).
/// Uses `to_tool = 3` (distinct from the base tool 0 and `from_tool = 0`) so a
/// regression to either the old hardcoded `0` or a `region_id`-derived value is
/// caught.
#[test]
fn purge_entities_carry_destination_tool_index() {
    let wt = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("wipe_tower_purge_volume", ConfigValue::Float(70.0)),
        ("wipe_tower_width", ConfigValue::Float(60.0)),
        ("line_width", ConfigValue::Float(0.4)),
        ("bed_shape", bed_shape_250()),
    ]);

    let layer = make_layer(0, 0.2, vec![tool_change(0, 0, 3)]);
    let views = vec![LayerCollectionView::new(layer)];
    let config = config_with(&[("bed_shape", bed_shape_250())]);
    let mut output = FinalizationOutputBuilder::new();
    wt.run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let purge_tools: Vec<u32> = output
        .merge_ops()
        .filter_map(|op| match op {
            MergeOp::InsertEntityAt {
                path, tool_index, ..
            } if matches!(path.role, ExtrusionRole::WipeTower) => Some(*tool_index),
            _ => None,
        })
        .collect();

    assert!(
        !purge_tools.is_empty(),
        "expected at least one wipe-tower purge insert"
    );
    assert!(
        purge_tools.iter().all(|&t| t == 3),
        "every purge entity must carry the destination tool (to_tool = 3); got {purge_tools:?}"
    );
}

// ─── AC-2: purge_volume controls entity-push count ───────────────────────────

#[test]
fn purge_volume_controls_finalization_push_count() {
    let wt_small = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("wipe_tower_purge_volume", ConfigValue::Float(70.0)),
        ("wipe_tower_width", ConfigValue::Float(60.0)),
        ("line_width", ConfigValue::Float(0.4)),
        ("bed_shape", bed_shape_250()),
    ]);
    let wt_large = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("wipe_tower_purge_volume", ConfigValue::Float(140.0)),
        ("wipe_tower_width", ConfigValue::Float(60.0)),
        ("line_width", ConfigValue::Float(0.4)),
        ("bed_shape", bed_shape_250()),
    ]);

    let layer_small = make_layer(0, 0.2, vec![tool_change(0, 0, 1)]);
    let layer_large = make_layer(0, 0.2, vec![tool_change(0, 0, 1)]);

    let views_small = vec![LayerCollectionView::new(layer_small)];
    let views_large = vec![LayerCollectionView::new(layer_large)];
    let config = config_with(&[("bed_shape", bed_shape_250())]);

    let mut out_small = FinalizationOutputBuilder::new();
    wt_small
        .run_finalization(&views_small, &mut out_small, &config)
        .expect("run_finalization must succeed");

    let mut out_large = FinalizationOutputBuilder::new();
    wt_large
        .run_finalization(&views_large, &mut out_large, &config)
        .expect("run_finalization must succeed");

    let smaller_inserts = wipe_tower_inserts_for_layer(&out_small, 0);
    let larger_inserts = wipe_tower_inserts_for_layer(&out_large, 0);
    assert!(
        larger_inserts.len() > smaller_inserts.len(),
        "larger purge volume ({}) should produce more wipe-tower inserts than smaller ({})",
        larger_inserts.len(),
        smaller_inserts.len()
    );
}

// ─── AC-3: only layers with tool changes are targeted ────────────────────────

#[test]
fn run_finalization_targets_only_layers_with_tool_changes() {
    let wt = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("bed_shape", bed_shape_250()),
    ]);

    // layer 0: no tool changes; layer 1: one tool change
    let layer0 = make_layer(0, 0.2, vec![]);
    let layer1 = make_layer(1, 0.4, vec![tool_change(0, 0, 1)]);
    let views = vec![
        LayerCollectionView::new(layer0),
        LayerCollectionView::new(layer1),
    ];
    let config = config_with(&[("bed_shape", bed_shape_250())]);
    let mut output = FinalizationOutputBuilder::new();
    wt.run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let targets = wipe_tower_inserts_total(&output);

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
    let config = config_with(&[("bed_shape", bed_shape_250())]);

    // Case A: wipe_tower_enabled=false with a layer that has a ToolChange
    let wt_disabled = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(false)),
        ("bed_shape", bed_shape_250()),
    ]);
    let layer_with_tc = make_layer(0, 0.2, vec![tool_change(0, 0, 1)]);
    let views_a = vec![LayerCollectionView::new(layer_with_tc)];
    let mut out_a = FinalizationOutputBuilder::new();
    wt_disabled
        .run_finalization(&views_a, &mut out_a, &config)
        .expect("must not error when disabled");
    assert!(
        out_a.entity_pushes().is_empty() && wipe_tower_inserts_total(&out_a).is_empty(),
        "disabled wipe tower must emit no entity pushes or inserts"
    );

    // Case B: wipe_tower_enabled=true but layer has NO tool_changes
    let wt_enabled = wipe_tower_from(&[
        ("wipe_tower_enabled", ConfigValue::Bool(true)),
        ("bed_shape", bed_shape_250()),
    ]);
    let layer_no_tc = make_layer(0, 0.2, vec![]);
    let views_b = vec![LayerCollectionView::new(layer_no_tc)];
    let mut out_b = FinalizationOutputBuilder::new();
    wt_enabled
        .run_finalization(&views_b, &mut out_b, &config)
        .expect("must not error when no tool changes");
    assert!(
        out_b.entity_pushes().is_empty() && wipe_tower_inserts_total(&out_b).is_empty(),
        "enabled wipe tower with no tool changes must emit no entity pushes or inserts"
    );
}
