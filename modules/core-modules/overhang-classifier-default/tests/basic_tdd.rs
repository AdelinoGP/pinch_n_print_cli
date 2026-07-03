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

/// Config with non-zero overhang speeds and base wall speeds. Also sets
/// `line_width` (matches `wall_square_with_quartile`'s hardcoded 0.4mm point
/// width) — required for the curl pathway to activate at all; without it
/// `flow_width` resolves to 0.0 and curl computation is a defensive no-op.
fn overhang_config() -> slicer_ir::ConfigView {
    ConfigViewBuilder::new()
        .float("outer_wall_speed", 60.0)
        .float("inner_wall_speed", 60.0)
        .float("thin_wall_speed", 60.0)
        .float("overhang_1_4_speed", 30.0)
        .float("overhang_2_4_speed", 40.0)
        .float("overhang_3_4_speed", 50.0)
        .float("overhang_4_4_speed", 60.0)
        .float("line_width", 0.4)
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

/// DEV-009 curled-edge slowdown: a wall directly above a previous-layer wall
/// that itself curled (via a small lateral offset from ITS OWN previous
/// layer) receives a `SetSpeedFactor` mutation driven purely by curl — every
/// vertex on all three layers carries `overhang_quartile: None`, isolating
/// the curl-only pathway from the pre-existing overhang pathway.
///
/// Three layers: layer 0 is the (curl-free, since it has no layer below)
/// reference geometry; layer 1 is offset 0.3mm in X from layer 0, which
/// falls inside the malformation distance band `(0.2, 1.1) * line_width`
/// (line_width = 0.4mm), so its own curled_height comes out positive; layer 2
/// sits at the SAME XY position as layer 1, well within `dist_limit = 10 *
/// line_width`, so it must observe layer 1's curl and slow down.
#[module_test]
fn curled_edge_triggers_slowdown_on_next_layer() {
    let cfg = overhang_config();
    let classifier = OverhangClassifierDefault::on_print_start(&cfg).unwrap();

    let layer0 = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.0, 0, 0, None);
    let layer1 = wall_square_with_quartile(1, 0.3, 0.0, 10.3, 10.0, 0.2, 0, 1, None);
    let layer2 = wall_square_with_quartile(1, 0.3, 0.0, 10.3, 10.0, 0.4, 0, 2, None);

    let views = vec![
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(0)
            .z(0.0)
            .add_entity(layer0)
            .build(),
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(1)
            .z(0.2)
            .add_entity(layer1)
            .build(),
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(2)
            .z(0.4)
            .add_entity(layer2)
            .build(),
    ]
    .into_iter()
    .map(LayerCollectionView::new)
    .collect::<Vec<_>>();

    let mut output = FinalizationOutputBuilder::new();
    classifier
        .run_finalization(&views, &mut output, &cfg)
        .expect("run_finalization must succeed");

    // Layer 0 and layer 1 must NOT receive a curl-driven mutation: layer 0 has
    // no layer below (curled_height forced to 0.0), and layer 1's only
    // reference (layer 0) has curled_height 0.0, so layer 1's own consumption
    // sees no curl signal either.
    let layer0_and_1_mutations: Vec<_> = output
        .merge_ops()
        .filter(|op| matches!(op, MergeOp::ModifyEntity { layer: 0 | 1, .. }))
        .collect();
    assert!(
        layer0_and_1_mutations.is_empty(),
        "expected no mutations on layers 0/1 (no curl to react to yet), got: {:?}",
        layer0_and_1_mutations
    );

    // Layer 2 must receive a SetSpeedFactor mutation driven by layer 1's curl.
    let layer2_speed_factors: Vec<f32> = output
        .merge_ops()
        .filter_map(|op| match op {
            MergeOp::ModifyEntity {
                layer: 2,
                entity_id: 1,
                mutation: EntityMutation::SetSpeedFactor(f),
            } => Some(*f),
            _ => None,
        })
        .collect();
    assert_eq!(
        layer2_speed_factors.len(),
        1,
        "expected exactly one curl-driven SetSpeedFactor mutation on layer 2"
    );
    assert!(
        layer2_speed_factors[0] < 1.0,
        "curl-driven speed factor must slow down the entity (factor < 1.0), got {}",
        layer2_speed_factors[0]
    );
}

/// Control for [`curled_edge_triggers_slowdown_on_next_layer`]: a wall far
/// away in XY from any curled geometry (well outside `dist_limit`) receives
/// no curl-driven mutation, even though a curled layer exists below it.
#[module_test]
fn curled_edge_out_of_range_emits_no_mutation() {
    let cfg = overhang_config();
    let classifier = OverhangClassifierDefault::on_print_start(&cfg).unwrap();

    let layer0 = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.0, 0, 0, None);
    let layer1 = wall_square_with_quartile(1, 0.3, 0.0, 10.3, 10.0, 0.2, 0, 1, None);
    // Layer 2 is far away in X (100mm offset) — well outside dist_limit
    // (10 * 0.4mm line width = 4.0mm).
    let layer2 = wall_square_with_quartile(1, 100.0, 0.0, 110.0, 10.0, 0.4, 0, 2, None);

    let views = vec![
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(0)
            .z(0.0)
            .add_entity(layer0)
            .build(),
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(1)
            .z(0.2)
            .add_entity(layer1)
            .build(),
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(2)
            .z(0.4)
            .add_entity(layer2)
            .build(),
    ]
    .into_iter()
    .map(LayerCollectionView::new)
    .collect::<Vec<_>>();

    let mut output = FinalizationOutputBuilder::new();
    classifier
        .run_finalization(&views, &mut output, &cfg)
        .expect("run_finalization must succeed");

    let layer2_mutations: Vec<_> = output
        .merge_ops()
        .filter(|op| matches!(op, MergeOp::ModifyEntity { layer: 2, .. }))
        .collect();
    assert!(
        layer2_mutations.is_empty(),
        "expected no mutation on a layer-2 wall far outside dist_limit, got: {:?}",
        layer2_mutations
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
