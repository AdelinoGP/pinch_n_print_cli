//! Library-validator membership contract, not a printable-geometry claim.

use proptest::prelude::*;
use slicer_ir::{
    validate_travel_anchors, ExtrusionRole, LayerCollectionIR, PrintEntity, TravelMove,
};
use slicer_sdk::test_support::fixtures::print_entity_base;
use slicer_sdk::test_support::property_testing::domains::{anchor_cases, AnchorDomain};
use slicer_sdk::test_support::property_testing::oracles::first_missing_anchor;
use slicer_sdk::test_support::property_testing::{
    check_travel_anchors_charter, travel_anchor_oracle, PropertyRun,
};

fn layer(entity_ids: &[u64], anchors: &[u64]) -> LayerCollectionIR {
    LayerCollectionIR {
        ordered_entities: entity_ids
            .iter()
            .map(|&entity_id| PrintEntity {
                entity_id,
                ..print_entity_base(ExtrusionRole::OuterWall)
            })
            .collect(),
        travel_moves: anchors
            .iter()
            .map(|&entity_id| TravelMove {
                entity_id,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn witness(entity_ids: &[u64], anchors: &[u64], first_missing: Option<u64>) {
    let layer = layer(entity_ids, anchors);
    // Hand-computed answers test the oracle as well as the real validator.
    assert_eq!(
        first_missing_anchor(&layer),
        first_missing,
        "oracle witness: entities={entity_ids:?}, anchors={anchors:?}"
    );
    check_travel_anchors_charter(&layer, validate_travel_anchors)
        .expect("real validator must satisfy the hand-computed membership witness");
}

#[test]
fn proptest_ir_travel_anchors_membership() {
    // Mandatory structural witnesses run on EVERY registry execution, before
    // random sampling. No probabilistic coverage assertion stands in for them.
    witness(&[], &[], None);
    witness(&[7], &[], None);
    witness(&[7], &[7], None);
    witness(&[], &[9], Some(9));
    witness(&[11], &[11, 1], Some(1));
    witness(&[7], &[7, 99, 2], Some(99)); // travel order, not sorted ID order
    witness(&[7], &[7, 7, 7], None);
    witness(&[7], &[9, 9], Some(9));
    witness(&[9, 2, 7], &[7, 9, 2], None);
    witness(&[2, 7, 9], &[7, 9, 2], None);
    witness(&[7, 7], &[7, 7], None);
    // Membership is not allocation/sentinel validation: both extremes can
    // belong, and both must also be rejected when absent.
    witness(&[0, u64::MAX], &[u64::MAX, 0, u64::MAX], None);
    witness(&[u64::MAX], &[0], Some(0));
    witness(&[0], &[u64::MAX], Some(u64::MAX));

    let run = PropertyRun::new("H-IR-ANCHORS", "proptest_ir_travel_anchors_membership")
        .expect("use the registry-backed property command");
    let max_entities = usize::try_from(run.bound("max_entities").expect("entity bound"))
        .expect("entity bound must fit usize");
    let max_travels = usize::try_from(run.bound("max_travels").expect("travel bound"))
        .expect("travel bound must fit usize");
    let cases = prop_oneof![
        anchor_cases(AnchorDomain::Valid, max_entities, max_travels).expect("valid bounds"),
        anchor_cases(AnchorDomain::Dangling, max_entities, max_travels).expect("dangling bounds"),
    ];
    run.check(&cases, |case, guard| {
        prop_assert_eq!(guard.entity_ids().next(), 1, "fresh per-case allocator");
        prop_assert_eq!(
            first_missing_anchor(&case.layer),
            travel_anchor_oracle(&case.layer)
                .err()
                .map(|error| error.entity_id)
        );
        check_travel_anchors_charter(&case.layer, validate_travel_anchors)
    })
    .expect("travel-anchor membership charter");
}
