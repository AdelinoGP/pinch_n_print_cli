//! Module-level TDD tests for the smoothed overhang-speed classifier.

#![allow(missing_docs)]

use slicer_ir::{ConfigView, ExtrusionRole, Point3WithWidth, PrintEntity, RegionKey};
use slicer_sdk::module_test;
use slicer_sdk::test_prelude::{print_entity, ConfigViewBuilder, LayerCollectionFixtureBuilder};
use slicer_sdk::traits::{
    EntityMutation, FinalizationModule, FinalizationOutputBuilder, LayerCollectionView, MergeOp,
};

use overhang_classifier_default::OverhangClassifierDefault;

const PATH_WIDTH: f32 = 0.4;

fn point(
    x: f32,
    y: f32,
    z: f32,
    overhang_quartile: Option<u8>,
    overhang_distance_mm: Option<f32>,
) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: PATH_WIDTH,
        flow_factor: 1.0,
        overhang_quartile,
        dist_to_top_mm: 0.0,
        overhang_distance_mm,
    }
}

fn square_points(
    z: f32,
    quartiles: [Option<u8>; 4],
    distances: [Option<f32>; 4],
) -> Vec<Point3WithWidth> {
    vec![
        point(0.0, 0.0, z, quartiles[0], distances[0]),
        point(10.0, 0.0, z, quartiles[1], distances[1]),
        point(10.0, 10.0, z, quartiles[2], distances[2]),
        point(0.0, 10.0, z, quartiles[3], distances[3]),
    ]
}

fn entity_with_points(
    entity_id: u64,
    role: ExtrusionRole,
    points: Vec<Point3WithWidth>,
    layer_index: u32,
    topo_order: u32,
) -> PrintEntity {
    print_entity(
        entity_id,
        role,
        points,
        RegionKey {
            global_layer_index: layer_index,
            object_id: "obj-0".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        topo_order,
    )
}

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
) -> PrintEntity {
    let w = PATH_WIDTH;
    let pt = |x: f32, y: f32| Point3WithWidth {
        x,
        y,
        z,
        width: w,
        flow_factor: 1.0,
        overhang_quartile: quartile,
        dist_to_top_mm: 0.0,
        overhang_distance_mm: None,
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

fn wall_square_with_quartile_and_distance(
    entity_id: u64,
    z: f32,
    layer_index: u32,
    quartile: Option<u8>,
    distance: Option<f32>,
) -> PrintEntity {
    entity_with_points(
        entity_id,
        ExtrusionRole::OuterWall,
        square_points(z, [quartile; 4], [distance; 4]),
        layer_index,
        0,
    )
}

fn wall_square_with_distances(
    entity_id: u64,
    z: f32,
    layer_index: u32,
    quartiles: [Option<u8>; 4],
    distances: [Option<f32>; 4],
) -> PrintEntity {
    entity_with_points(
        entity_id,
        ExtrusionRole::OuterWall,
        square_points(z, quartiles, distances),
        layer_index,
        0,
    )
}

fn base_overhang_config() -> ConfigViewBuilder {
    ConfigViewBuilder::new()
        .float("outer_wall_speed", 60.0)
        .float("inner_wall_speed", 60.0)
        .float("thin_wall_speed", 60.0)
        .float("overhang_1_4_speed", 30.0)
        .float("overhang_2_4_speed", 40.0)
        .float("overhang_3_4_speed", 50.0)
        .float("overhang_4_4_speed", 60.0)
        .float("bridge_speed", 25.0)
        .float("line_width", f64::from(PATH_WIDTH))
}

/// Config with non-zero overhang speeds and the canonical bridge branch.
/// `enable_overhang_speed` is intentionally absent so its default is tested.
fn overhang_config() -> ConfigView {
    base_overhang_config()
        .bool("slowdown_for_curled_perimeters", false)
        .build()
}

fn overhang_config_with_q4_speed(q4_speed: f64) -> ConfigView {
    base_overhang_config()
        .float("overhang_4_4_speed", q4_speed)
        .bool("slowdown_for_curled_perimeters", false)
        .build()
}

fn run_classifier(views: &[LayerCollectionView], config: &ConfigView) -> FinalizationOutputBuilder {
    let classifier = OverhangClassifierDefault::from_config(config).unwrap();
    let mut output = FinalizationOutputBuilder::new();
    classifier
        .run_finalization(views, &mut output, config)
        .expect("run_finalization must succeed");
    output
}

fn point_speed_profiles(
    output: &FinalizationOutputBuilder,
    layer: u32,
    entity_id: u64,
) -> Vec<Vec<f32>> {
    output
        .merge_ops()
        .filter_map(|op| match op {
            MergeOp::ModifyEntity {
                layer: op_layer,
                entity_id: op_entity_id,
                mutation: EntityMutation::SetPointSpeedFactors(factors),
            } if *op_layer == layer && *op_entity_id == entity_id => Some(factors.clone()),
            _ => None,
        })
        .collect()
}

fn two_layer_views(upper_entity: PrintEntity) -> Vec<LayerCollectionView> {
    let lower_entity = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.0, 0, 0, None);
    vec![
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(0)
            .z(0.0)
            .add_entity(lower_entity)
            .build(),
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(1)
            .z(0.2)
            .add_entity(upper_entity)
            .build(),
    ]
    .into_iter()
    .map(LayerCollectionView::new)
    .collect()
}

#[module_test]
fn quartile_present_receives_speed_factor_below_one() {
    let cfg = overhang_config();
    let entity = wall_square_with_quartile_and_distance(1, 0.2, 1, Some(3), Some(0.25));
    let views = two_layer_views(entity);
    let output = run_classifier(&views, &cfg);

    let mutation_factors = output
        .merge_ops()
        .find_map(|op| match op {
            MergeOp::ModifyEntity {
                layer,
                entity_id,
                mutation: EntityMutation::SetPointSpeedFactors(factors),
            } if *layer == 1 && *entity_id == 1 => Some(factors.clone()),
            _ => None,
        })
        .expect("expected SetPointSpeedFactors for layer 1 entity 1");
    assert_eq!(mutation_factors.len(), 4);
    assert!(mutation_factors
        .iter()
        .all(|factor| (*factor - 45.0 / 60.0).abs() < 1e-6));

    let profiles = point_speed_profiles(&output, 1, 1);
    assert_eq!(
        profiles.len(),
        1,
        "expected exactly one point-speed mutation"
    );
    assert_eq!(profiles[0].len(), 4);
    for factor in &profiles[0] {
        assert!((*factor - 45.0 / 60.0).abs() < 1e-6);
        assert!(*factor < 1.0);
    }
}

#[module_test]
fn quartile_absent_emits_no_mutation() {
    let cfg = overhang_config();
    let entity = wall_square_with_quartile_and_distance(1, 0.2, 1, None, Some(0.25));
    let views = two_layer_views(entity);
    let output = run_classifier(&views, &cfg);

    assert_eq!(
        output.merge_ops().count(),
        0,
        "expected no mutations when overhang_quartile is None"
    );
}

#[module_test]
fn quartile_four_is_honored() {
    let cfg = overhang_config_with_q4_speed(20.0);
    let entity = wall_square_with_quartile_and_distance(1, 0.2, 1, Some(4), Some(0.348));
    let views = two_layer_views(entity);
    let output = run_classifier(&views, &cfg);

    let mutation_factors = output
        .merge_ops()
        .find_map(|op| match op {
            MergeOp::ModifyEntity {
                layer,
                entity_id,
                mutation: EntityMutation::SetPointSpeedFactors(factors),
            } if *layer == 1 && *entity_id == 1 => Some(factors.clone()),
            _ => None,
        })
        .expect("expected SetPointSpeedFactors for layer 1 entity 1");
    assert_eq!(mutation_factors.len(), 4);
    assert!(mutation_factors
        .iter()
        .all(|factor| (*factor - 20.0 / 60.0).abs() < 1e-6));

    let profiles = point_speed_profiles(&output, 1, 1);
    assert_eq!(
        profiles.len(),
        1,
        "expected exactly one point-speed mutation"
    );
    assert_eq!(profiles[0].len(), 4);
    for factor in &profiles[0] {
        assert!((*factor - 20.0 / 60.0).abs() < 1e-6);
    }
}

#[module_test]
fn curled_edge_triggers_slowdown_on_next_layer() {
    let cfg = overhang_config();

    let layer0 = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.0, 0, 0, None);
    let layer1 = wall_square_with_quartile(1, 0.3, 0.0, 10.3, 10.0, 0.2, 0, 1, None);
    let layer2 = wall_square_with_quartile(1, 0.3, 0.0, 10.3, 10.0, 0.4, 0, 2, Some(1));

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

    let output = run_classifier(&views, &cfg);

    let layer0_and_1_mutations: Vec<_> = output
        .merge_ops()
        .filter(|op| matches!(op, MergeOp::ModifyEntity { layer: 0 | 1, .. }))
        .collect();
    assert!(
        layer0_and_1_mutations.is_empty(),
        "expected no mutations on layers 0/1 (no curl to react to yet), got: {:?}",
        layer0_and_1_mutations
    );

    let mutation_factors = output
        .merge_ops()
        .find_map(|op| match op {
            MergeOp::ModifyEntity {
                layer,
                entity_id,
                mutation: EntityMutation::SetPointSpeedFactors(factors),
            } if *layer == 2 && *entity_id == 1 => Some(factors.clone()),
            _ => None,
        })
        .expect("expected SetPointSpeedFactors for layer 2 entity 1");
    assert_eq!(mutation_factors.len(), 4);
    assert!(mutation_factors.iter().all(|factor| factor.is_finite()));
    assert!(mutation_factors.iter().any(|factor| *factor < 1.0));

    let profiles = point_speed_profiles(&output, 2, 1);
    assert_eq!(
        profiles.len(),
        1,
        "expected exactly one curl-driven point-speed mutation on layer 2"
    );
    assert_eq!(profiles[0].len(), 4);
    assert!(profiles[0].iter().all(|factor| factor.is_finite()));
    assert!(
        profiles[0].iter().any(|factor| *factor < 1.0),
        "curl-driven factors must slow down at least one point: {:?}",
        profiles[0]
    );
}

#[module_test]
fn curled_edge_out_of_range_emits_no_mutation() {
    let cfg = overhang_config();

    let layer0 = wall_square_with_quartile(1, 0.0, 0.0, 10.0, 10.0, 0.0, 0, 0, None);
    let layer1 = wall_square_with_quartile(1, 0.3, 0.0, 10.3, 10.0, 0.2, 0, 1, None);
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

    let output = run_classifier(&views, &cfg);

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
    let entity = wall_square_with_quartile_and_distance(1, 0.2, 1, Some(2), Some(0.25));
    let views = two_layer_views(entity);
    let output = run_classifier(&views, &cfg);

    assert_eq!(
        output.merge_ops().count(),
        0,
        "expected no mutations when all overhang speeds are 0.0"
    );
}

#[module_test]
fn calculate_speed_matches_canonical_interpolation_and_clamps() {
    let cfg = overhang_config();
    let sections = overhang_classifier_default::build_speed_sections(60.0, PATH_WIDTH, &cfg);

    assert_eq!(sections.len(), 6);
    assert!((sections[0].0 - 0.04).abs() < 1e-6);
    assert!((sections.last().unwrap().0 - 0.4).abs() < 1e-6);

    assert_eq!(
        overhang_classifier_default::calculate_speed(0.03, &sections, 40.0),
        40.0
    );
    assert_eq!(
        overhang_classifier_default::calculate_speed(-0.1, &sections, 40.0),
        40.0
    );
    assert_eq!(
        overhang_classifier_default::calculate_speed(sections[0].0, &sections, 40.0),
        40.0
    );
    assert_eq!(
        overhang_classifier_default::calculate_speed(0.4, &sections, 40.0),
        25.0
    );
    assert_eq!(
        overhang_classifier_default::calculate_speed(0.5, &sections, 40.0),
        25.0
    );

    let distance: f32 = 0.15;
    let t: f32 = (distance - 0.1_f32) / (0.2_f32 - 0.1_f32);
    let expected: f32 = ((1.0_f32 - t) * 30.0_f32 + t * 40.0_f32).round();
    assert_eq!(
        overhang_classifier_default::calculate_speed(distance, &sections, 40.0),
        expected
    );

    for distance in [
        -1.0, 0.03, 0.04, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.34, 0.35, 0.4, 0.5,
    ] {
        assert!(
            overhang_classifier_default::calculate_speed(distance, &sections, 40.0) <= 40.0,
            "speed exceeded original speed at distance {distance}"
        );
    }
}

#[module_test]
fn sixth_speed_section_follows_slowdown_for_curled_perimeters() {
    let false_sections =
        overhang_classifier_default::build_speed_sections(60.0, PATH_WIDTH, &overhang_config());
    assert_eq!(false_sections.last().unwrap().1, 25.0);

    let true_cfg = base_overhang_config()
        .float("overhang_4_4_speed", 45.0)
        .bool("slowdown_for_curled_perimeters", true)
        .build();
    let true_sections =
        overhang_classifier_default::build_speed_sections(60.0, PATH_WIDTH, &true_cfg);
    assert_eq!(true_sections.last().unwrap().1, 45.0);

    let guarded_cfg = base_overhang_config()
        .float("overhang_4_4_speed", 0.0)
        .bool("slowdown_for_curled_perimeters", true)
        .build();
    let guarded_sections =
        overhang_classifier_default::build_speed_sections(60.0, PATH_WIDTH, &guarded_cfg);
    assert_eq!(guarded_sections.last().unwrap().1, 60.0);
}

#[module_test]
fn section_speeds_resolve_against_ref_speed_not_original_speed() {
    let cfg = overhang_config();
    let original_speed: f32 = 40.0;
    let changed_original_speed: f32 = 100.0;
    let sections_for_original_speed =
        overhang_classifier_default::build_speed_sections(60.0, PATH_WIDTH, &cfg);
    let sections_for_changed_original_speed =
        overhang_classifier_default::build_speed_sections(60.0, PATH_WIDTH, &cfg);

    assert_eq!(sections_for_original_speed[0].1, 60.0);
    assert_eq!(sections_for_original_speed[5].1, 25.0);
    assert_eq!(
        sections_for_original_speed,
        sections_for_changed_original_speed
    );

    let clamped = overhang_classifier_default::calculate_speed(
        0.3,
        &sections_for_original_speed,
        original_speed,
    );
    let unclamped = overhang_classifier_default::calculate_speed(
        0.3,
        &sections_for_changed_original_speed,
        changed_original_speed,
    );
    assert!(clamped <= original_speed);
    assert_eq!(unclamped, 50.0);
}

#[module_test]
fn speed_sections_flatten_ties_without_removing_entries() {
    let cfg = overhang_config();
    let sections = overhang_classifier_default::build_speed_sections(60.0, 0.0, &cfg);

    assert_eq!(
        sections.len(),
        overhang_classifier_default::OVERHANG_OVERLAP_LEVELS.len()
    );
    assert!(sections.iter().all(|(distance, _)| *distance == 0.0));
    assert!(
        sections.iter().all(|(_, speed)| *speed == 60.0),
        "equal-distance entries must retain the earlier higher speed: {:?}",
        sections
    );
}

#[module_test]
fn per_point_factors_vary_within_one_entity() {
    let cfg = overhang_config();
    let upper = wall_square_with_distances(
        1,
        0.2,
        1,
        [Some(1), Some(1), Some(1), Some(1)],
        [Some(-0.1), Some(0.15), Some(0.25), None],
    );
    let views = two_layer_views(upper);
    let output = run_classifier(&views, &cfg);

    assert_eq!(output.merge_ops().count(), 1);
    let profiles = point_speed_profiles(&output, 1, 1);
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].len(), 4);
    assert_eq!(profiles[0][3], 1.0, "None distance must remain full speed");
    let first_factor = profiles[0][0];
    assert!(
        profiles[0].iter().all(|factor| factor.is_finite())
            && profiles[0]
                .iter()
                .any(|factor| (*factor - first_factor).abs() > 1e-6),
        "expected at least two distinct finite factors: {:?}",
        profiles[0]
    );
}

#[module_test]
fn interpolated_factor_is_not_a_quartile_value() {
    let cfg = overhang_config();
    let upper = wall_square_with_distances(
        1,
        0.2,
        1,
        [Some(1), Some(1), Some(1), Some(1)],
        [Some(0.15), Some(-0.1), Some(-0.1), Some(-0.1)],
    );
    let views = two_layer_views(upper);
    let output = run_classifier(&views, &cfg);
    let profiles = point_speed_profiles(&output, 1, 1);

    assert_eq!(profiles.len(), 1);
    let t: f32 = (0.15 - 0.1) / (0.2 - 0.1);
    let expected_factor: f32 = ((1.0 - t) * 30.0 + t * 40.0).round() / 60.0;
    assert!((profiles[0][0] - expected_factor).abs() < 1e-6);
    assert!((profiles[0][0] - 30.0 / 60.0).abs() > 1e-6);
    assert!((profiles[0][0] - 40.0 / 60.0).abs() > 1e-6);
}

#[module_test]
fn enable_overhang_speed_false_disables_all_mutations_and_absent_defaults_true() {
    let disabled_cfg = base_overhang_config()
        .bool("slowdown_for_curled_perimeters", false)
        .bool("enable_overhang_speed", false)
        .build();
    let disabled_upper = wall_square_with_quartile_and_distance(1, 0.2, 1, Some(2), Some(0.25));
    let disabled_output = run_classifier(&two_layer_views(disabled_upper), &disabled_cfg);
    assert_eq!(disabled_output.merge_ops().count(), 0);

    let absent_cfg = overhang_config();
    let absent_upper = wall_square_with_quartile_and_distance(1, 0.2, 1, Some(2), Some(0.25));
    let absent_output = run_classifier(&two_layer_views(absent_upper), &absent_cfg);
    assert_eq!(point_speed_profiles(&absent_output, 1, 1).len(), 1);
}

#[module_test]
fn first_layer_emits_no_speed_mutation() {
    let cfg = overhang_config();
    let entity = wall_square_with_quartile_and_distance(1, 0.0, 0, Some(2), Some(0.25));
    let layer = LayerCollectionFixtureBuilder::new()
        .global_layer_index(0)
        .z(0.0)
        .add_entity(entity)
        .build();
    let views = vec![LayerCollectionView::new(layer)];

    let output = run_classifier(&views, &cfg);
    assert_eq!(output.merge_ops().count(), 0);
}

#[module_test]
fn non_wall_role_emits_no_mutation_and_no_nan() {
    let cfg = overhang_config();
    let upper = entity_with_points(
        1,
        ExtrusionRole::SparseInfill,
        square_points(
            0.2,
            [Some(2), Some(2), Some(2), Some(2)],
            [Some(0.25), Some(0.25), Some(0.25), Some(0.25)],
        ),
        1,
        0,
    );
    let views = two_layer_views(upper);
    let output = run_classifier(&views, &cfg);

    for op in output.merge_ops() {
        if let MergeOp::ModifyEntity {
            mutation: EntityMutation::SetPointSpeedFactors(factors),
            ..
        } = op
        {
            assert!(factors.iter().all(|factor| factor.is_finite()));
        }
    }
    assert_eq!(output.merge_ops().count(), 0);
}
