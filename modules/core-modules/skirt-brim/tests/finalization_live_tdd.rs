#![allow(missing_docs)]

use std::collections::HashMap;

use skirt_brim::SkirtBrim;
use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer,
};
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

// ---- Helpers ----

fn semver() -> SemVer {
    SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

fn make_entity_at(layer_index: u32, x: f32, y: f32, z: f32) -> PrintEntity {
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
            global_layer_index: layer_index,
            object_id: "obj1".to_string(),
            region_id: 1,
        },
        topo_order: 0,
    }
}

fn make_layer(index: u32, z: f32, entities: Vec<PrintEntity>) -> LayerCollectionIR {
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

fn config_with(pairs: &[(&str, ConfigValue)]) -> ConfigView {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v.clone());
    }
    ConfigView::from_map(map)
}

fn skirt_brim_from(pairs: &[(&str, ConfigValue)]) -> SkirtBrim {
    SkirtBrim::from_config(&config_with(pairs)).expect("config should be valid")
}

// ─── AC-1: skirt pushes on targeted layers ───────────────────────────────────

#[test]
fn run_finalization_pushes_skirt_entities_to_target_layers() {
    let sb = skirt_brim_from(&[
        ("skirt_loops", ConfigValue::Int(2)),
        ("skirt_height", ConfigValue::Int(1)),
        ("brim_width", ConfigValue::Float(0.0)),
    ]);

    let layer = make_layer(
        0,
        0.2,
        vec![
            make_entity_at(0, 10.0, 10.0, 0.2),
            make_entity_at(0, 20.0, 20.0, 0.2),
        ],
    );
    let views = vec![LayerCollectionView::new(layer)];
    let config = config_with(&[]);
    let mut output = FinalizationOutputBuilder::new();
    sb.run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let pushes = output.entity_pushes();
    assert_eq!(
        pushes.len(),
        2,
        "expected exactly 2 skirt pushes for skirt_loops=2"
    );
    for (layer_index, path, region_key) in pushes {
        assert_eq!(*layer_index, 0, "skirt must target layer 0");
        assert_eq!(path.role, ExtrusionRole::Skirt, "path role must be Skirt");
        assert_eq!(
            region_key.object_id, "__skirt__",
            "object_id must be '__skirt__'"
        );
    }
}

// ─── AC-2: brim pushes on layer 0 only via push_entity_to_layer ──────────────

#[test]
fn run_finalization_pushes_brim_entities_on_layer_zero_only() {
    let sb = skirt_brim_from(&[
        ("brim_width", ConfigValue::Float(3.0)),
        ("skirt_loops", ConfigValue::Int(0)),
    ]);

    let layer = make_layer(
        0,
        0.2,
        vec![
            make_entity_at(0, 10.0, 10.0, 0.2),
            make_entity_at(0, 20.0, 20.0, 0.2),
        ],
    );
    let views = vec![LayerCollectionView::new(layer)];
    let config = config_with(&[]);
    let mut output = FinalizationOutputBuilder::new();
    sb.run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let pushes = output.entity_pushes();
    assert!(!pushes.is_empty(), "brim pushes must not be empty");
    for (layer_index, path, region_key) in pushes {
        assert_eq!(*layer_index, 0, "brim must target layer 0 only");
        assert_eq!(
            path.role,
            ExtrusionRole::Skirt,
            "brim path role must be Skirt"
        );
        assert_eq!(
            region_key.object_id, "__brim__",
            "object_id must be '__brim__'"
        );
    }
    assert!(
        output.synthetic_layers().is_empty(),
        "brim must not use insert_synthetic_layer"
    );
}

// ─── AC-3: skirt_height restricts layer targeting ────────────────────────────

#[test]
fn run_finalization_respects_skirt_height_layer_targeting() {
    let sb = skirt_brim_from(&[
        ("skirt_height", ConfigValue::Int(3)),
        ("skirt_loops", ConfigValue::Int(1)),
    ]);

    let layers: Vec<LayerCollectionIR> = (0..4u32)
        .map(|i| {
            make_layer(
                i,
                (i as f32 + 1.0) * 0.2,
                vec![make_entity_at(i, 10.0, 10.0, (i as f32 + 1.0) * 0.2)],
            )
        })
        .collect();
    let views: Vec<LayerCollectionView> =
        layers.into_iter().map(LayerCollectionView::new).collect();
    let config = config_with(&[]);
    let mut output = FinalizationOutputBuilder::new();
    sb.run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let pushes = output.entity_pushes();
    let targeted: Vec<u32> = pushes.iter().map(|(li, _, _)| *li).collect();
    assert!(targeted.contains(&0), "layer 0 must be targeted");
    assert!(targeted.contains(&1), "layer 1 must be targeted");
    assert!(targeted.contains(&2), "layer 2 must be targeted");
    assert!(
        !targeted.contains(&3),
        "layer 3 must not be targeted with skirt_height=3"
    );
}

// ─── AC-Neg: disabled or empty input emits no pushes ─────────────────────────

#[test]
fn disabled_or_empty_input_emits_no_finalization_pushes() {
    let config = config_with(&[]);

    // Case 1: skirt_brim_enabled=false
    let sb_disabled = skirt_brim_from(&[("skirt_brim_enabled", ConfigValue::Bool(false))]);
    let layer = make_layer(0, 0.2, vec![make_entity_at(0, 10.0, 10.0, 0.2)]);
    let views = vec![LayerCollectionView::new(layer)];
    let mut out1 = FinalizationOutputBuilder::new();
    sb_disabled
        .run_finalization(&views, &mut out1, &config)
        .expect("must not error");
    assert!(
        out1.entity_pushes().is_empty(),
        "disabled: no entity pushes"
    );
    assert!(
        out1.synthetic_layers().is_empty(),
        "disabled: no synthetic layers"
    );

    // Case 2: empty layer set
    let sb = skirt_brim_from(&[]);
    let views_empty: Vec<LayerCollectionView> = vec![];
    let mut out2 = FinalizationOutputBuilder::new();
    sb.run_finalization(&views_empty, &mut out2, &config)
        .expect("must not error");
    assert!(
        out2.entity_pushes().is_empty(),
        "empty input: no entity pushes"
    );
    assert!(
        out2.synthetic_layers().is_empty(),
        "empty input: no synthetic layers"
    );
}
