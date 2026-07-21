//! Behavioral tests for the part-cooling `FinalizationModule`.
//!
//! Drives `run_finalization` against a real `FinalizationOutputBuilder` and
//! inspects the recorded fan-command annotations. Host G-code *rendering* of
//! these annotations is covered by slicer-runtime's emitter tests; here we test
//! the module's decision logic: first-layers disable, overhang bump, trailing
//! fan-off, and the `fan_speed_max == 0` short-circuit.

#![allow(missing_docs)]

use part_cooling::PartCooling;
use slicer_ir::{
    ConfigValue, ExtrusionPath3D, ExtrusionRole, LayerAnnotationKind, LayerCollectionIR,
    Point3WithWidth, PrintEntity, RegionKey, SemVer,
};
use slicer_sdk::test_prelude::config_with;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn entity(role: ExtrusionRole) -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            }],
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        tool_index: 0,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "obj".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    }
}

fn layer_view(index: u32, roles: &[ExtrusionRole]) -> LayerCollectionView {
    let ir = LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: index,
        z: 0.2 * (index + 1) as f32,
        ordered_entities: roles.iter().cloned().map(entity).collect(),
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    };
    LayerCollectionView::new(ir)
}

/// Raw annotation bodies recorded for `layer`, in record order.
fn raws_for_layer(output: &FinalizationOutputBuilder, layer: u32) -> Vec<String> {
    output
        .annotations()
        .iter()
        .filter(|(l, _)| *l == layer)
        .filter_map(|(_, a)| match &a.kind {
            LayerAnnotationKind::Raw(s) => Some(s.clone()),
            LayerAnnotationKind::Comment(_) => None,
        })
        .collect()
}

fn run(
    config_pairs: &[(&str, ConfigValue)],
    layers: &[LayerCollectionView],
) -> FinalizationOutputBuilder {
    let cfg = config_with(config_pairs);
    let module = PartCooling::from_config(&cfg).expect("config must be valid");
    let mut output = FinalizationOutputBuilder::new();
    module
        .run_finalization(layers, &mut output, &cfg)
        .expect("run_finalization must succeed");
    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn first_layers_disabled_then_fan_on() {
    let layers = vec![
        layer_view(0, &[ExtrusionRole::OuterWall]),
        layer_view(1, &[ExtrusionRole::OuterWall]),
        layer_view(2, &[ExtrusionRole::OuterWall]),
    ];
    let output = run(
        &[
            ("fan_speed_max", ConfigValue::Int(255)),
            ("disable_fan_first_layers", ConfigValue::Int(2)),
            ("enable_overhang_fan", ConfigValue::Bool(false)),
        ],
        &layers,
    );

    let l0 = raws_for_layer(&output, 0);
    assert!(
        l0.contains(&"M107".to_string()),
        "layer 0 must be fan-off (M107): {l0:?}"
    );
    assert!(
        !l0.iter().any(|s| s.starts_with("M106 S") && s != "M106 S0"),
        "layer 0 must not turn the fan on: {l0:?}"
    );
    let l1 = raws_for_layer(&output, 1);
    assert!(
        l1.contains(&"M107".to_string()),
        "layer 1 must be fan-off (M107): {l1:?}"
    );
    let l2 = raws_for_layer(&output, 2);
    assert!(
        l2.contains(&"M106 S255".to_string()),
        "layer 2 (>= disable_fan_first_layers) must turn fan to max: {l2:?}"
    );
}

#[test]
fn trailing_fan_off_after_last_layer() {
    let layers = vec![
        layer_view(0, &[ExtrusionRole::OuterWall]),
        layer_view(1, &[ExtrusionRole::OuterWall]),
    ];
    let output = run(
        &[
            ("fan_speed_max", ConfigValue::Int(255)),
            ("disable_fan_first_layers", ConfigValue::Int(0)),
            ("enable_overhang_fan", ConfigValue::Bool(false)),
        ],
        &layers,
    );

    // The fan-off after the final layer is anchored after the last entity.
    let last_layer_off = output
        .annotations()
        .iter()
        .filter(|(l, _)| *l == 1)
        .any(|(_, a)| {
            a.after_entity_index == u32::MAX
                && matches!(&a.kind, LayerAnnotationKind::Raw(s) if s == "M107")
        });
    assert!(
        last_layer_off,
        "a trailing M107 (after_entity_index = u32::MAX) must close the print on the last layer"
    );
}

#[test]
fn overhang_region_bumps_fan() {
    // overhang_fan_speed=40, fan_speed_max=255 → bump value = 40*255/100 = 102.
    // Base/restore stay at 255, so the bump is distinguishable as M106 S102.
    let layers = vec![layer_view(
        0,
        &[
            ExtrusionRole::OuterWall,
            ExtrusionRole::BridgeInfill,
            ExtrusionRole::OuterWall,
        ],
    )];
    let output = run(
        &[
            ("fan_speed_max", ConfigValue::Int(255)),
            ("disable_fan_first_layers", ConfigValue::Int(0)),
            ("enable_overhang_fan", ConfigValue::Bool(true)),
            ("overhang_fan_speed", ConfigValue::Int(40)),
        ],
        &layers,
    );

    let l0 = raws_for_layer(&output, 0);
    assert!(
        l0.contains(&"M106 S102".to_string()),
        "overhang entity must bump the fan to 102 (40% of 255): {l0:?}"
    );
    assert!(
        l0.iter().filter(|s| *s == "M106 S255").count() >= 2,
        "base + restore must both set the fan back to 255: {l0:?}"
    );
}

#[test]
fn overhang_fan_disabled_no_bump() {
    let layers = vec![layer_view(
        0,
        &[ExtrusionRole::OuterWall, ExtrusionRole::BridgeInfill],
    )];
    let output = run(
        &[
            ("fan_speed_max", ConfigValue::Int(255)),
            ("disable_fan_first_layers", ConfigValue::Int(0)),
            ("enable_overhang_fan", ConfigValue::Bool(false)),
            ("overhang_fan_speed", ConfigValue::Int(40)),
        ],
        &layers,
    );

    let l0 = raws_for_layer(&output, 0);
    assert!(
        !l0.contains(&"M106 S102".to_string()),
        "with enable_overhang_fan=false there must be no overhang bump: {l0:?}"
    );
}

#[test]
fn fan_speed_max_zero_emits_single_m107() {
    let layers = vec![
        layer_view(0, &[ExtrusionRole::OuterWall]),
        layer_view(1, &[ExtrusionRole::OuterWall]),
    ];
    let output = run(
        &[
            ("fan_speed_max", ConfigValue::Int(0)),
            ("disable_fan_first_layers", ConfigValue::Int(1)),
        ],
        &layers,
    );

    let all = output.annotations();
    assert_eq!(
        all.len(),
        1,
        "fan_speed_max=0 must emit exactly one annotation, got {}",
        all.len()
    );
    assert!(
        matches!(&all[0].1.kind, LayerAnnotationKind::Raw(s) if s == "M107"),
        "the single annotation must be M107"
    );
    assert_eq!(all[0].0, 0, "the M107 must land on the first layer");
}

#[test]
fn empty_layers_is_noop() {
    let layers: Vec<LayerCollectionView> = vec![];
    let output = run(
        &[
            ("fan_speed_max", ConfigValue::Int(255)),
            ("disable_fan_first_layers", ConfigValue::Int(1)),
        ],
        &layers,
    );
    assert!(
        output.annotations().is_empty(),
        "no layers must produce no fan annotations"
    );
}
