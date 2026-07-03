//! Module-level TDD test for overhang-classifier-default (AC-8).
//!
//! Post-refactor: this module is a pure consumer of the per-vertex
//! `overhang_quartile` field already written onto `Point3WithWidth` by the
//! upstream PrePass::OverhangAnnotation pipeline (packet 106 / ADR-0031). It
//! performs NO local geometric computation, so these tests set
//! `overhang_quartile` directly on fixture points rather than constructing
//! two-layer geometry for a distance-based classifier.

#![allow(missing_docs)]

use slicer_ir::{ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::module_test;
use slicer_sdk::test_prelude::{print_entity, ConfigViewBuilder, LayerCollectionFixtureBuilder};
use slicer_sdk::traits::{
    EntityMutation, FinalizationModule, FinalizationOutputBuilder, LayerCollectionView, MergeOp,
};

use overhang_classifier_default::OverhangClassifierDefault;

/// Helper: build a `PrintEntity` with an OuterWall role and a single explicit
/// `overhang_quartile` value applied to every vertex of a rectangular path.
fn wall_square_with_quartile(
    entity_id: u64,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    z: f32,
    topo_order: u32,
    layer_index: u32,
    quartile: Option<u8>,
) -> slicer_ir::PrintEntity {
    let w = 0.4_f32;
    let pt = |x: f32, y: f32| Point3WithWidth {
        x,
        y,
        z,
        width: w,
        flow_factor: 1.0,
        overhang_quartile: quartile,
    };
    print_entity(
        entity_id,
        ExtrusionRole::OuterWall,
        vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)],
        RegionKey {
            global_layer_index: layer_index,
            object_id: "obj-0".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
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

/// A wall entity with `overhang_quartile = Some(3)` on every vertex receives
/// a `SetSpeedFactor` mutation of `overhang_3_4_speed / outer_wall_speed`
/// (50.0 / 60.0).
#[module_test]
fn quartile_present_receives_speed_factor_below_one() {
    let cfg = overhang_config();
    let classifier = OverhangClassifierDefault::on_print_start(&cfg).unwrap();

    let entity = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.4, 0, 1, Some(3));
    let layer = LayerCollectionFixtureBuilder::new()
        .global_layer_index(1)
        .z(0.4)
        .add_entity(entity)
        .build();

    let views = vec![LayerCollectionView::new(layer)];
    let mut output = FinalizationOutputBuilder::new();

    classifier
        .run_finalization(&views, &mut output, &cfg)
        .expect("run_finalization must succeed");

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

    assert_eq!(
        speed_factors,
        vec![50.0_f32 / 60.0_f32],
        "expected exactly one SetSpeedFactor mutation matching overhang_3_4_speed / outer_wall_speed"
    );
}

/// A wall entity whose vertices all carry `overhang_quartile = None` (not
/// classified upstream) receives no mutation at all.
#[module_test]
fn quartile_absent_emits_no_mutation() {
    let cfg = overhang_config();
    let classifier = OverhangClassifierDefault::on_print_start(&cfg).unwrap();

    let entity = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.2, 0, 0, None);
    let layer = LayerCollectionFixtureBuilder::new()
        .global_layer_index(0)
        .z(0.2)
        .add_entity(entity)
        .build();

    let views = vec![LayerCollectionView::new(layer)];
    let mut output = FinalizationOutputBuilder::new();

    classifier
        .run_finalization(&views, &mut output, &cfg)
        .expect("run_finalization must succeed");

    assert_eq!(
        output.merge_ops().count(),
        0,
        "expected no mutations when overhang_quartile is None"
    );
}

/// Quartile 4 ("worst") is honored post-refactor (unlike the pre-refactor
/// algorithm, which unconditionally skipped quartile >= 4 and left
/// `overhang_4_4_speed` dead config). This is an intentional, packet-approved
/// behavior delta — see packet 107 notes.
#[module_test]
fn quartile_four_is_honored() {
    let cfg = overhang_config();
    let classifier = OverhangClassifierDefault::on_print_start(&cfg).unwrap();

    let entity = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.4, 0, 1, Some(4));
    let layer = LayerCollectionFixtureBuilder::new()
        .global_layer_index(1)
        .z(0.4)
        .add_entity(entity)
        .build();

    let views = vec![LayerCollectionView::new(layer)];
    let mut output = FinalizationOutputBuilder::new();

    classifier
        .run_finalization(&views, &mut output, &cfg)
        .expect("run_finalization must succeed");

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

    assert_eq!(
        speed_factors,
        vec![60.0_f32 / 60.0_f32],
        "expected a SetSpeedFactor mutation matching overhang_4_4_speed / outer_wall_speed"
    );
}

/// All-zero overhang-speed config yields the early return: zero mutations,
/// even when vertices carry a classified quartile.
#[module_test]
fn all_zero_config_emits_no_mutations() {
    let cfg = ConfigViewBuilder::new()
        .float("outer_wall_speed", 60.0)
        .float("inner_wall_speed", 60.0)
        .float("thin_wall_speed", 60.0)
        .float("overhang_1_4_speed", 0.0)
        .float("overhang_2_4_speed", 0.0)
        .float("overhang_3_4_speed", 0.0)
        .float("overhang_4_4_speed", 0.0)
        .build();
    let classifier = OverhangClassifierDefault::on_print_start(&cfg).unwrap();

    let entity = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.4, 0, 1, Some(2));
    let layer = LayerCollectionFixtureBuilder::new()
        .global_layer_index(1)
        .z(0.4)
        .add_entity(entity)
        .build();

    let views = vec![LayerCollectionView::new(layer)];
    let mut output = FinalizationOutputBuilder::new();

    classifier
        .run_finalization(&views, &mut output, &cfg)
        .expect("run_finalization must succeed");

    assert_eq!(
        output.merge_ops().count(),
        0,
        "expected no mutations when all overhang speeds are 0.0"
    );
}
