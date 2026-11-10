//! Library-validator membership contract, not a printable-geometry claim.

use proptest::prelude::*;
use slicer_ir::{
    validate_travel_anchors, ExtrusionRole, LayerCollectionIR, PrintEntity, TravelMove,
};
use slicer_sdk::test_support::fixtures::print_entity_base;
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

fn id_strategy() -> impl Strategy<Value = u64> {
    // The small domain makes duplicates/membership common; unrestricted IDs
    // and the maximum exercise the full integer range without an allocator.
    prop_oneof![3 => 0_u64..=15, 1 => Just(u64::MAX), 4 => any::<u64>()]
}

fn anchor_cases(
    max_entities: usize,
    max_travels: usize,
) -> impl Strategy<Value = (Vec<u64>, Vec<u64>)> {
    (
        prop::collection::vec(id_strategy(), 0..=max_entities),
        prop::collection::vec((any::<usize>(), id_strategy()), 0..=max_travels),
        any::<bool>(),
    )
        .prop_map(|(entities, raw_anchors, select_members)| {
            let anchors = raw_anchors
                .into_iter()
                .map(|(index, arbitrary)| {
                    if select_members && !entities.is_empty() {
                        entities[index % entities.len()]
                    } else {
                        arbitrary
                    }
                })
                .collect();
            (entities, anchors)
        })
}

fn witness(entity_ids: &[u64], anchors: &[u64], first_missing: Option<u64>) {
    let layer = layer(entity_ids, anchors);
    // Hand-computed answers test the oracle as well as the real validator.
    assert_eq!(
        travel_anchor_oracle(&layer)
            .err()
            .map(|error| error.entity_id),
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
    run.check(
        &anchor_cases(max_entities, max_travels),
        |(entities, anchors), guard| {
            prop_assert_eq!(guard.entity_ids().next(), 1, "fresh per-case allocator");
            check_travel_anchors_charter(&layer(&entities, &anchors), validate_travel_anchors)
        },
    )
    .expect("travel-anchor membership charter");
}
