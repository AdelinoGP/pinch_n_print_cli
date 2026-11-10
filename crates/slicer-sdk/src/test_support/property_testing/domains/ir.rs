//! Membership, entity-layer and order strategies.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, LayerEntityIdGen, Point3WithWidth,
    PrintEntity, TravelMove,
};

use super::{AnchorCase, AnchorDomain, DomainError, EntityDomain};
use crate::test_support::fixtures::{extrusion_path3d_base, print_entity_base};

const MAX_COLLECTION: usize = 4096;
const MAX_LAYER_DIMENSION: usize = 128;
const MAX_POINTS_TOTAL: usize = 100_000;

fn ids() -> impl Strategy<Value = u64> {
    prop_oneof![2 => Just(0), 2 => Just(u64::MAX), 3 => 0_u64..=15, 3 => any::<u64>()]
}

pub(super) fn anchor_cases(
    domain: AnchorDomain,
    max_entities: usize,
    max_travels: usize,
) -> Result<BoxedStrategy<AnchorCase>, DomainError> {
    if max_entities > MAX_COLLECTION
        || max_travels > MAX_COLLECTION
        || (domain == AnchorDomain::Dangling && max_travels == 0)
    {
        return Err(DomainError::InvalidBounds(
            "anchor bounds are excessive or cannot retain a dangling travel".into(),
        ));
    }
    let prefix_max = max_travels - usize::from(domain == AnchorDomain::Dangling);
    Ok((
        proptest::collection::vec(ids(), 0..=max_entities),
        proptest::collection::vec(any::<usize>(), 0..=prefix_max),
    )
        .prop_map(move |(entities, choices)| {
            let mut anchors: Vec<_> = if entities.is_empty() {
                Vec::new()
            } else {
                choices
                    .into_iter()
                    .map(|index| entities[index % entities.len()])
                    .collect()
            };
            if domain == AnchorDomain::Dangling {
                let existing: BTreeSet<_> = entities.iter().copied().collect();
                let missing = [0, u64::MAX]
                    .into_iter()
                    .find(|id| !existing.contains(id))
                    .unwrap_or_else(|| {
                        (1_u64..)
                            .find(|id| !existing.contains(id))
                            .expect("bounded entities leave unused IDs")
                    });
                anchors.push(missing);
            }
            AnchorCase {
                layer: LayerCollectionIR {
                    ordered_entities: entities
                        .into_iter()
                        .map(|entity_id| PrintEntity {
                            entity_id,
                            ..print_entity_base(ExtrusionRole::OuterWall)
                        })
                        .collect(),
                    travel_moves: anchors
                        .into_iter()
                        .map(|entity_id| TravelMove {
                            entity_id,
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                },
            }
        })
        .boxed())
}

pub(super) fn entity_layers(
    domain: EntityDomain,
) -> Result<BoxedStrategy<Vec<LayerCollectionIR>>, DomainError> {
    if domain.roles.is_empty() {
        return Err(DomainError::InvalidInput(
            "at least one role is required".into(),
        ));
    }
    if domain.max_entities > MAX_LAYER_DIMENSION
        || domain.max_points > MAX_LAYER_DIMENSION
        || domain.max_layers > MAX_LAYER_DIMENSION
        || domain
            .max_layers
            .checked_mul(domain.max_entities)
            .and_then(|n| n.checked_mul(domain.max_points))
            .is_none_or(|n| n > MAX_POINTS_TOTAL)
    {
        return Err(DomainError::InvalidBounds(
            "entity layer bounds are excessive".into(),
        ));
    }
    let point = (
        -128_i16..=128,
        -128_i16..=128,
        -128_i16..=128,
        1_u8..=16,
        1_u8..=16,
    )
        .prop_map(|(x, y, z, width, flow)| Point3WithWidth {
            x: f32::from(x) / 8.0,
            y: f32::from(y) / 8.0,
            z: f32::from(z) / 8.0,
            width: f32::from(width) / 8.0,
            flow_factor: f32::from(flow) / 8.0,
            ..Default::default()
        });
    let points = if domain.max_points == 0 {
        Just(Vec::new()).boxed()
    } else {
        proptest::collection::vec(point, 1..=domain.max_points).boxed()
    };
    let entity = (proptest::sample::select(domain.roles), points, 1_u8..=16);
    let max_entities = if domain.max_points == 0 {
        0
    } else {
        domain.max_entities
    };
    let layers = proptest::collection::vec(
        proptest::collection::vec(entity, 0..=max_entities),
        0..=domain.max_layers,
    );
    Ok(layers
        .prop_map(|layers| {
            let ids = LayerEntityIdGen::new();
            layers
                .into_iter()
                .enumerate()
                .map(|(index, entities)| LayerCollectionIR {
                    global_layer_index: u32::try_from(index).expect("checked layer limit fits u32"),
                    z: index as f32 * 0.125,
                    ordered_entities: entities
                        .into_iter()
                        .map(|(role, points, speed)| PrintEntity {
                            entity_id: ids.next(),
                            path: ExtrusionPath3D {
                                points,
                                speed_factor: f32::from(speed) / 8.0,
                                ..extrusion_path3d_base(role.clone())
                            },
                            ..print_entity_base(role)
                        })
                        .collect(),
                    ..Default::default()
                })
                .collect()
        })
        .boxed())
}

pub(super) fn permutations(len: usize) -> BoxedStrategy<Vec<usize>> {
    // Proptest's exact-size range cannot represent usize::MAX. Defer all
    // allocation for this otherwise-unrepresentable size until sampling.
    if len == usize::MAX {
        return any::<u64>()
            .prop_map(move |seed| {
                use std::hash::{Hash, Hasher};
                let mut indices: Vec<_> = (0..len).collect();
                indices.sort_unstable_by_key(|index| {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    seed.hash(&mut hasher);
                    index.hash(&mut hasher);
                    (hasher.finish(), *index)
                });
                indices
            })
            .boxed();
    }
    proptest::collection::vec(any::<u64>(), len)
        .prop_map(|keys| {
            let mut indices: Vec<_> = (0..keys.len()).collect();
            indices.sort_unstable_by_key(|&index| (keys[index], index));
            indices
        })
        .boxed()
}

pub(super) fn locked_orders(
    len: usize,
    groups: Vec<Vec<usize>>,
) -> Result<BoxedStrategy<Vec<usize>>, DomainError> {
    if groups.len() > len {
        return Err(DomainError::InvalidInput(
            "more lock groups than indices".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for group in &groups {
        if group.is_empty()
            || group
                .iter()
                .any(|&index| index >= len || !seen.insert(index))
        {
            return Err(DomainError::InvalidInput(
                "lock groups must be nonempty, disjoint and in range".into(),
            ));
        }
    }
    if len > MAX_COLLECTION {
        return Err(DomainError::InvalidBounds(
            "lock order limit exceeds 4096".into(),
        ));
    }
    let mut blocks = groups;
    blocks.extend(
        (0..len)
            .filter(|index| !seen.contains(index))
            .map(|index| vec![index]),
    );
    Ok(permutations(blocks.len())
        .prop_map(move |order| {
            order
                .into_iter()
                .flat_map(|index| blocks[index].iter().copied())
                .collect()
        })
        .boxed())
}
