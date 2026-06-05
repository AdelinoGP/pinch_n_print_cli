//! Module-level TDD test for overhang-classifier-default (AC-8).
//!
//! Verifies that wall entities overhanging the previous layer receive a
//! `SetSpeedFactor` mutation with factor < 1.0.

#![allow(missing_docs)]

use slicer_ir::{ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::module_test;
use slicer_sdk::test_prelude::{print_entity, ConfigViewBuilder, LayerCollectionFixtureBuilder};
use slicer_sdk::traits::{
    EntityMutation, FinalizationModule, FinalizationOutputBuilder, LayerCollectionView, MergeOp,
};

use overhang_classifier_default::OverhangClassifierDefault;

/// Helper: build a `PrintEntity` with an OuterWall role forming a closed
/// rectangular polygon from four corners.
fn wall_square(
    entity_id: u64,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    z: f32,
    topo_order: u32,
    layer_index: u32,
) -> slicer_ir::PrintEntity {
    let w = 0.4_f32;
    print_entity(
        entity_id,
        ExtrusionRole::OuterWall,
        vec![
            Point3WithWidth {
                x: x0,
                y: y0,
                z,
                width: w,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: x1,
                y: y0,
                z,
                width: w,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: x1,
                y: y1,
                z,
                width: w,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: x0,
                y: y1,
                z,
                width: w,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
        ],
        RegionKey {
            global_layer_index: layer_index,
            object_id: "obj-0".to_string(),
            region_id: 0,
        },
        topo_order,
    )
}

/// Config with non-zero overhang speeds and base wall speeds.
fn overhang_config() -> slicer_ir::ConfigView {
    ConfigViewBuilder::new()
        .float("outer_wall_speed", 60.0)
        .float("inner_wall_speed", 60.0)
        .float("thin_wall_speed", 60.0)
        .float("overhang_1_4_speed", 30.0)
        .float("overhang_2_4_speed", 40.0)
        .float("overhang_3_4_speed", 50.0)
        .float("overhang_4_4_speed", 60.0)
        .build()
}

/// Two-layer overhang fixture: layer 0 is a 10×10 supported square,
/// layer 1 is a slightly expanded 10.1×10.1 square whose wall points
/// sit just outside the layer-0 polygon (negative signed distance → Q3).
///
/// With width 0.4, Q3 threshold is sd > -0.25 * 0.4 = -0.1.
/// The 0.05 mm outward offset produces sd ≈ -0.05 → Q3 → overhang_3_4_speed / base_speed.
#[module_test]
fn overhang_wall_receives_speed_factor_below_one() {
    let cfg = overhang_config();
    let classifier = OverhangClassifierDefault::on_print_start(&cfg).unwrap();

    // Layer 0: supported 10×10 square wall at z = 0.2
    let layer0_entity = wall_square(1, 0.0, 0.0, 10.0, 10.0, 0.2, 0, 0);
    let layer0 = LayerCollectionFixtureBuilder::new()
        .global_layer_index(0)
        .z(0.2)
        .add_entity(layer0_entity)
        .build();

    // Layer 1: 10.1×10.1 square wall — 0.05mm outward offset on each side.
    // Points lie outside the layer-0 polygon → negative sd → classified as overhang.
    let layer1_entity = wall_square(1, -0.05, -0.05, 10.05, 10.05, 0.4, 0, 1);
    let layer1 = LayerCollectionFixtureBuilder::new()
        .global_layer_index(1)
        .z(0.4)
        .add_entity(layer1_entity)
        .build();

    let views = vec![
        LayerCollectionView::new(layer0),
        LayerCollectionView::new(layer1),
    ];
    let mut output = FinalizationOutputBuilder::new();

    classifier
        .run_finalization(&views, &mut output, &cfg)
        .expect("run_finalization must succeed");

    // Collect SetSpeedFactor mutations targeting layer 1, entity 1.
    let speed_factors: Vec<f32> = output
        .merge_ops()
        .filter_map(|op| match op {
            MergeOp::ModifyEntity {
                layer: 1,
                entity_id: 1,
                mutation: EntityMutation::SetSpeedFactor(f),
            } => Some(*f),
            _ => None,
        })
        .collect();

    assert!(
        !speed_factors.is_empty(),
        "expected at least one SetSpeedFactor mutation for the overhanging layer-1 entity"
    );
    for f in &speed_factors {
        assert!(
            *f < 1.0,
            "speed factor must be < 1.0 for overhanging entity, got {f}"
        );
    }
}
