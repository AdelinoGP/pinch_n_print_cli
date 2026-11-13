//! Library-validator membership contract plus the bounded coordinate and ID
//! charters, not a printable-geometry claim.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use slicer_ir::region_split_registry::enumerate_canonical_chains;
use slicer_ir::{
    is_modifier_namespace_id, mm_to_units, modifier_base_region_id, modifier_sub_region_id,
    modifier_sub_region_id_fits, validate_travel_anchors, ConfigId, ConfigValue,
    EntitySpeedProfile, ExPolygon, ExtrusionRole, LayerCollectionIR, LayerEntityIdGen, PaintValue,
    Point2, PrintEntity, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SupportEntityIdentity,
    TravelMove, MODIFIER_FOOTPRINT_REGION_ID, MODIFIER_VARIANT_REGION_ID_STRIDE, UNITS_PER_MM,
};
use slicer_sdk::coords::SCALING_FACTOR;
use slicer_sdk::test_support::fixtures::print_entity_base;
use slicer_sdk::test_support::property_testing::domains::{
    anchor_cases, entity_layers, finite_f32, id_text, nonfinite_f64, rounding_boundary_mm,
    scalar_config_values, AnchorDomain, DomainError, EntityDomain, IdTextKind, ScalarConfigDomain,
};
use slicer_sdk::test_support::property_testing::oracles::first_missing_anchor;
use slicer_sdk::test_support::property_testing::{
    check_travel_anchors_charter, travel_anchor_oracle, PropertyRun,
};

#[path = "common/property_oracles.rs"]
#[allow(dead_code)]
mod property_oracles;

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

/// Deliberately wrong reference: unions `ordered_entities` with every unrelated
/// side table that carries an `entity_id`. This is the exact defect the real
/// validator must not have, so the side-table witness asserts that it answers
/// `None` where the production validator and the independent oracle both
/// report a dangling anchor.
fn side_table_union_first_missing(layer: &LayerCollectionIR) -> Option<u64> {
    let mut ids: BTreeSet<u64> = layer
        .ordered_entities
        .iter()
        .map(|entity| entity.entity_id)
        .collect();
    ids.extend(layer.speed_profiles.iter().map(|profile| profile.entity_id));
    ids.extend(
        layer
            .support_entity_identities
            .iter()
            .map(|identity| identity.entity_id),
    );
    layer
        .travel_moves
        .iter()
        .map(|travel| travel.entity_id)
        .find(|id| !ids.contains(id))
}

/// Whether `id` appears in `message` as a standalone decimal token, rather than
/// as a digit sequence embedded in a longer number.
fn names_standalone_decimal(message: &str, id: u64) -> bool {
    let text = id.to_string();
    message
        .split(|c: char| !c.is_ascii_digit())
        .any(|token| token == text)
}

/// A travel anchor that exists only in an unrelated side table is still
/// dangling: `validate_travel_anchors` reads `layer.ordered_entities` alone.
fn side_table_only_anchor_witness() {
    let travel_id = 9_u64;
    // `property_oracles::layer_with_side_table_only_anchor` puts the travel id
    // in `travel_moves` and `speed_profiles` only, never in `ordered_entities`.
    let layer = property_oracles::layer_with_side_table_only_anchor(&[7], travel_id);
    assert!(
        layer
            .ordered_entities
            .iter()
            .all(|entity| entity.entity_id != travel_id),
        "the side-table witness must keep the anchor out of layer.ordered_entities"
    );
    assert_eq!(first_missing_anchor(&layer), Some(travel_id));
    let error = validate_travel_anchors(&layer)
        .expect_err("a travel anchor present only in a side table must not be accepted");
    assert!(
        names_standalone_decimal(&error, travel_id),
        "error must name the missing travel id {travel_id} as a standalone decimal token: {error}"
    );
    assert!(
        error.contains("layer.ordered_entities"),
        "error must name the membership source: {error}"
    );
    // The controlled mutation accepts the anchor because `speed_profiles`
    // mentions it; both the real validator and the oracle disagree, so the
    // mutation is caught.
    assert_eq!(
        side_table_union_first_missing(&layer),
        None,
        "the side-table-union mutation must wrongly accept the anchor"
    );
    assert_ne!(
        side_table_union_first_missing(&layer),
        first_missing_anchor(&layer),
        "the mutation must be distinguishable from the real answer"
    );
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
    // A missing id that exists only in an unrelated side table is still
    // missing; the diagnostic must not be satisfiable from that table.
    side_table_only_anchor_witness();

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

// ============================================================================
// H-COORD: bounded conversion envelopes, not fictional exact inverses
// ============================================================================

/// One generated coordinate case. The variant tag is structural, so shrinking
/// cannot turn a millimetre case into a unit case or into an unordered pair.
#[derive(Debug, Clone)]
enum CoordCase {
    /// A bounded millimetre input for `slicer_ir::mm_to_units`.
    Millimetres(f32),
    /// A bounded integer-unit input for `slicer_ir::units_to_mm`.
    Units(i64),
    /// An ordered (`left <= right`) millimetre pair for monotonicity.
    OrderedMillimetres(f32, f32),
}

/// Largest f32 at or below the profile's `max_abs_mm`, so narrowing the f64
/// profile bound to f32 can never widen the declared domain.
fn mm_bound_f32(max_abs_mm: f64) -> f32 {
    let narrowed = max_abs_mm as f32;
    if f64::from(narrowed) > max_abs_mm {
        narrowed.next_down()
    } else {
        narrowed
    }
}

fn coord_cases(
    profile: &property_oracles::CoordProfile,
    boundary_units: i64,
) -> BoxedStrategy<CoordCase> {
    let max_abs_mm = mm_bound_f32(profile.max_abs_mm);
    let millimetres =
        finite_f32(-max_abs_mm..=max_abs_mm).expect("bounded finite f32 millimetre domain");
    let boundaries = rounding_boundary_mm(boundary_units)
        .expect("shared rounding-boundary domain inside the charter bound");
    let units = (-profile.max_abs_units..=profile.max_abs_units).prop_map(CoordCase::Units);
    let ordered_pairs = (millimetres.clone(), millimetres.clone()).prop_map(|(left, right)| {
        if left <= right {
            CoordCase::OrderedMillimetres(left, right)
        } else {
            CoordCase::OrderedMillimetres(right, left)
        }
    });
    prop_oneof![
        4 => millimetres.prop_map(CoordCase::Millimetres),
        2 => boundaries.prop_map(CoordCase::Millimetres),
        3 => units,
        3 => ordered_pairs,
    ]
    .boxed()
}

#[test]
fn proptest_coord_conversion_envelopes() {
    let run = PropertyRun::new("H-COORD", "proptest_coord_conversion_envelopes")
        .expect("use the registry-backed property command");
    let max_abs_mm = run.bound("max_abs_mm").expect("millimetre bound") as f64;
    let max_abs_units = i64::try_from(run.bound("max_abs_units").expect("unit bound"))
        .expect("the unit bound must fit i64");
    // Unsafe profiles are rejected, never clamped: a rejection is a property
    // failure naming the violated inequality.
    let profile = match property_oracles::CoordProfile::new(max_abs_mm, max_abs_units) {
        Ok(profile) => profile,
        Err(error) => panic!("H-COORD bounds do not form a safe profile: {error}"),
    };
    // The shared `rounding_boundary_mm` strategy accepts at most 1_000_000
    // units, and `CoordProfile::new` already guarantees that this `min(...)`
    // stays inside that cap.
    let boundary_units = (10_000.0 * max_abs_mm) as i64;
    let boundary_units = boundary_units.min(max_abs_units);
    assert!(
        boundary_units >= 1,
        "the shared rounding-boundary strategy needs a positive unit bound, got {boundary_units}"
    );

    // --- mandatory witnesses, on every execution ---------------------------
    // 1/32 mm is exactly 312.5 units, so half-away rounding must produce 313
    // and its negative mirror -313; the derivation in `expected_half_away` must
    // predict the same pair.
    let half = 1.0f32 / 32.0;
    assert_eq!(
        mm_to_units(half),
        313,
        "the exact half-unit input must round away from zero"
    );
    assert_eq!(
        mm_to_units(-half),
        -313,
        "the negative mirror must round away from zero"
    );
    assert_eq!(property_oracles::expected_half_away(half), 313);
    assert_eq!(property_oracles::expected_half_away(-half), -313);

    // Neighbour floats pin the tie direction through the same envelope-plus-tie
    // checker the sampled cases use.
    for neighbour in [
        half.next_down(),
        half,
        half.next_up(),
        (-half).next_down(),
        -half,
        (-half).next_up(),
    ] {
        if let Err(error) = property_oracles::check_mm_case_with(&profile, neighbour, mm_to_units) {
            panic!(
                "half-case neighbour {neighbour:?} left the derived envelope or tie rule: {error}"
            );
        }
    }

    // Both zero signs map to integer zero.
    assert_eq!(mm_to_units(0.0), 0);
    assert_eq!(mm_to_units(-0.0), 0);

    // Subnormals and the interval ends, through the full consistency checker.
    let interval_end = mm_bound_f32(profile.max_abs_mm);
    for candidate in [
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        interval_end,
        -interval_end,
    ] {
        if let Err(error) = property_oracles::check_mm_case(&profile, candidate) {
            panic!("mm witness {candidate:?} failed the derived envelope: {error}");
        }
    }

    // Derived unit envelope at both signs, the sentinels and the interval ends.
    for units in [-profile.max_abs_units, -1, 0, 1, profile.max_abs_units] {
        if let Err(error) = property_oracles::check_units_case(&profile, units) {
            panic!("unit witness {units} failed the derived envelope: {error}");
        }
    }

    // Monotonicity across the whole domain.
    if let Err(error) =
        property_oracles::check_monotonic_pair(&profile, -interval_end, interval_end)
    {
        panic!("monotonicity failed at the interval ends: {error}");
    }

    // The scaling factor is the single documented constant on both sides.
    assert_eq!(UNITS_PER_MM, 10_000.0);
    assert_eq!(SCALING_FACTOR, 10_000);

    // Exact integer `Point2` identity, never through a float round trip.
    let extreme = Point2 {
        x: i64::MIN,
        y: i64::MAX,
    };
    let bytes = postcard::to_allocvec(&extreme).expect("Point2 is Postcard-serializable");
    let decoded: Point2 = postcard::from_bytes(&bytes).expect("Point2 integer bytes decode");
    assert_eq!(
        decoded, extreme,
        "Point2 integer identity must survive Postcard exactly"
    );

    // --- sampled cases -----------------------------------------------------
    // The charters deliberately do not assert an inverse. Composing the
    // `0.5 + 2^-24*|10_000*m|` unit budget with the `2^-24*|n|/10_000` mm budget
    // leaves a non-empty interval, and 1/32 mm -> 313 units -> 0.0313 mm shows
    // the composition is not the identity.
    let cases = coord_cases(&profile, boundary_units);
    run.check(&cases, |case, _guard| {
        let checked = match case {
            CoordCase::Millimetres(millimetres) => {
                property_oracles::check_mm_case(&profile, millimetres)
            }
            CoordCase::Units(units) => property_oracles::check_units_case(&profile, units),
            CoordCase::OrderedMillimetres(left, right) => {
                property_oracles::check_monotonic_pair(&profile, left, right)
            }
        };
        if let Err(error) = checked {
            prop_assert!(false, "{}", error);
        }
        Ok(())
    })
    .expect("coordinate conversion charter");
}

// ============================================================================
// H-IDS: bounded allocation and representable modifier parents
// ============================================================================

/// One generated ID case. The variant tag is structural, so shrinking cannot
/// move a supported parent into the unsupported arm, nor an allocation length
/// into a modifier case.
#[derive(Debug, Clone)]
enum IdCase {
    /// A bounded number of allocations from a fresh generator.
    Allocations(u64),
    /// A parent inside the representable range, with bounded text and
    /// integer-grid footprint inputs.
    SupportedModifier {
        parent: u64,
        object_id: String,
        footprint: Vec<ExPolygon>,
    },
    /// A parent outside the representable range: `modifier_sub_region_id_fits`
    /// must refuse it. Minting it anyway is an assertion, observed once by the
    /// deterministic contract target rather than per generated case.
    UnsupportedParent {
        /// Parent identifier rejected by the public representability predicate.
        parent: u64,
    },
}

/// Integer-grid footprints with at most `max_points` total points.
///
/// The empty case (no points at all) and a holed shape arise from the same
/// split: the first `contour_len` points form the contour and every remaining
/// point joins one hole ring, so the total never exceeds the bound. No float is
/// involved, so the geometry is exactly reproducible and shrinking stays inside
/// the bound.
fn footprint_cases(max_points: usize) -> BoxedStrategy<Vec<ExPolygon>> {
    let grid = 0_i64..=64;
    (
        0_usize..=max_points,
        proptest::collection::vec((grid.clone(), grid), 0..=max_points),
    )
        .prop_map(move |(contour_len, points)| {
            let contour_len = contour_len.min(points.len());
            let (contour, rest) = points.split_at(contour_len);
            let holes = if rest.is_empty() {
                Vec::new()
            } else {
                vec![rest.to_vec()]
            };
            property_oracles::footprint(contour, &holes)
        })
        .boxed()
}

fn id_cases(
    max_allocations: u64,
    max_id_bytes: usize,
    max_footprint_points: usize,
    parent_limit: u64,
) -> BoxedStrategy<IdCase> {
    // `id_text`'s UUID family needs 36 bytes; the complement family covers
    // narrower future bounds without weakening the current ones.
    let text_kind = if max_id_bytes >= 36 {
        IdTextKind::UuidSyntax
    } else {
        IdTextKind::OtherText
    };
    prop_oneof![
        3 => (0..=max_allocations).prop_map(IdCase::Allocations),
        4 => (
            0..=parent_limit,
            id_text(text_kind, max_id_bytes).expect("bounded identifier-text domain"),
            footprint_cases(max_footprint_points),
        )
            .prop_map(|(parent, object_id, footprint)| IdCase::SupportedModifier {
                parent,
                object_id,
                footprint,
            }),
        2 => {
            // Both entries are outside the representable range, so shrinking
            // within this strategy keeps the unsupported mode.
            proptest::sample::select(vec![parent_limit + 1, u64::MAX])
                .prop_map(|parent| IdCase::UnsupportedParent { parent })
        }
    ]
    .boxed()
}

#[test]
fn proptest_entity_and_modifier_ids() {
    let run = PropertyRun::new("H-IDS", "proptest_entity_and_modifier_ids")
        .expect("use the registry-backed property command");
    let max_allocations = run.bound("max_allocations").expect("allocation bound");
    let max_id_bytes = usize::try_from(run.bound("max_id_bytes").expect("identifier byte bound"))
        .expect("the identifier byte bound must fit usize");
    let max_footprint_points =
        usize::try_from(run.bound("max_footprint_points").expect("footprint bound"))
            .expect("the footprint bound must fit usize");

    let stride = MODIFIER_VARIANT_REGION_ID_STRIDE;
    let parent_limit = property_oracles::representable_parent_limit();

    // --- mandatory allocation witnesses, on every execution ----------------
    let allocate = |count: u64| -> Vec<u64> {
        let generator = LayerEntityIdGen::new();
        (0..count).map(|_| generator.next()).collect()
    };
    assert_eq!(
        allocate(0),
        Vec::<u64>::new(),
        "a fresh generator allocates nothing for zero requests"
    );
    assert_eq!(allocate(1), vec![1], "the first fresh identifier is 1");
    for count in 1..=64_u64 {
        let actual = allocate(count);
        assert_eq!(
            actual,
            property_oracles::allocation_model(count),
            "{count} fresh allocations must equal the mathematical sequence 1..={count}"
        );
        assert!(
            !actual.contains(&0),
            "the reserved zero sentinel is never allocated"
        );
    }
    assert_eq!(
        allocate(16),
        allocate(16),
        "two fresh generators must produce the same sequence"
    );

    // --- derived representable-parent boundary -----------------------------
    assert!(
        modifier_sub_region_id_fits(parent_limit),
        "the derived limit P must be representable"
    );
    assert!(
        !modifier_sub_region_id_fits(parent_limit + 1),
        "P + 1 must not be representable"
    );
    assert!(
        !modifier_sub_region_id_fits(u64::MAX),
        "u64::MAX must not be representable"
    );
    for parent in [
        0,
        1,
        2,
        stride - 1,
        stride,
        stride + 1,
        parent_limit - 1,
        parent_limit,
    ] {
        assert_eq!(
            property_oracles::payload_fits_model(parent),
            modifier_sub_region_id_fits(parent),
            "the independent payload model must agree with the production predicate at {parent}"
        );
    }
    assert_eq!(
        property_oracles::payload_fits_model(parent_limit + 1),
        modifier_sub_region_id_fits(parent_limit + 1),
        "the model must agree with the production predicate just past P"
    );

    // --- supported-parent namespace, payload and decode --------------------
    let footprints = [
        property_oracles::footprint(&[], &[]),
        property_oracles::footprint(
            &[(0, 0), (20, 0), (20, 20), (0, 20)],
            &[vec![(5, 5), (6, 5), (5, 6)]],
        ),
    ];
    let objects = ["obj", "modifier-alpha"];
    for parent in [0_u64, 1, 2, stride, parent_limit] {
        assert!(modifier_sub_region_id_fits(parent));
        for (index, footprint) in footprints.iter().enumerate() {
            let object_id = objects[index];
            let id = modifier_sub_region_id(parent, object_id, footprint);
            assert_eq!(id >> 63, 1, "minted ids carry the namespace marker");
            assert_ne!(id, MODIFIER_FOOTPRINT_REGION_ID);
            assert!(is_modifier_namespace_id(id));
            assert_eq!(modifier_base_region_id(id), Some(parent));
            let (quotient, remainder) =
                property_oracles::decode_model(id).expect("a minted id decodes");
            assert_eq!(quotient, parent);
            assert!(
                (1..stride).contains(&remainder),
                "payload remainder {remainder} must lie in 1..{stride}"
            );
            assert_eq!(
                id,
                modifier_sub_region_id(parent, object_id, footprint),
                "identical inputs must mint identical ids"
            );
        }
    }

    // Distinct parents cannot share an id for identical object/footprint
    // inputs. Different object/footprint inputs CAN collide inside the bounded
    // stride, so no universal injectivity (or unequal-hash) claim is made.
    let shared = &footprints[1];
    let shared_ids: Vec<u64> = [0_u64, 1, 2, stride, parent_limit]
        .iter()
        .map(|&parent| modifier_sub_region_id(parent, objects[0], shared))
        .collect();
    for (index, left) in shared_ids.iter().enumerate() {
        for right in &shared_ids[index + 1..] {
            assert_ne!(left, right, "distinct parents must not share a minted id");
        }
    }

    // --- sentinel and namespace membership --------------------------------
    assert_eq!(modifier_base_region_id(MODIFIER_FOOTPRINT_REGION_ID), None);
    assert_eq!(
        property_oracles::decode_model(MODIFIER_FOOTPRINT_REGION_ID),
        None
    );
    for ordinary in [0_u64, 1, 12_345, u64::MAX >> 1] {
        assert_eq!(modifier_base_region_id(ordinary), None);
        assert_eq!(property_oracles::decode_model(ordinary), None);
    }
    // Namespace membership is not provenance validation: an arbitrary
    // bit-marked value decodes even though it was never minted.
    let never_minted = (1_u64 << 63) | (7 * stride + 3);
    assert!(is_modifier_namespace_id(never_minted));
    assert_eq!(modifier_base_region_id(never_minted), Some(7));
    assert_eq!(property_oracles::decode_model(never_minted), Some((7, 3)));

    // --- sampled cases -----------------------------------------------------
    let cases = id_cases(
        max_allocations,
        max_id_bytes,
        max_footprint_points,
        parent_limit,
    );
    run.check(&cases, |case, guard| {
        match case {
            IdCase::Allocations(count) => {
                let actual: Vec<u64> = (0..count).map(|_| guard.entity_ids().next()).collect();
                prop_assert_eq!(
                    actual,
                    property_oracles::allocation_model(count),
                    "{} fresh allocations must equal the mathematical sequence 1..=n",
                    count
                );
            }
            IdCase::SupportedModifier {
                parent,
                object_id,
                footprint,
            } => {
                prop_assert!(
                    modifier_sub_region_id_fits(parent),
                    "generated parent {} must be representable",
                    parent
                );
                let id = modifier_sub_region_id(parent, &object_id, &footprint);
                prop_assert_eq!(id >> 63, 1, "minted ids carry the namespace marker");
                prop_assert_ne!(id, MODIFIER_FOOTPRINT_REGION_ID);
                prop_assert!(is_modifier_namespace_id(id));
                prop_assert_eq!(modifier_base_region_id(id), Some(parent));
                let decoded = property_oracles::decode_model(id);
                prop_assert!(decoded.is_some(), "a minted id must land in the namespace");
                if let Some((quotient, remainder)) = decoded {
                    prop_assert_eq!(quotient, parent);
                    prop_assert!(
                        (1..MODIFIER_VARIANT_REGION_ID_STRIDE).contains(&remainder),
                        "payload remainder {} must lie in 1..{}",
                        remainder,
                        MODIFIER_VARIANT_REGION_ID_STRIDE
                    );
                }
                prop_assert_eq!(
                    id,
                    modifier_sub_region_id(parent, &object_id, &footprint),
                    "identical inputs must mint identical ids"
                );
            }
            IdCase::UnsupportedParent { parent } => {
                // The public predicate is the rejection seam: an unsupported
                // parent is refused here. Minting it anyway aborts the process
                // through `assert!`, so the documented assertion text is
                // observed once, deterministically, by
                // `ir_id_parent_and_sentinel_rejections`
                // (`crates/slicer-ir/tests/property_ir_contracts.rs`) instead
                // of being re-triggered for every generated case. That keeps
                // this property's runner log free of panic output while still
                // exercising the boundary across the whole sampled domain.
                prop_assert!(
                    !modifier_sub_region_id_fits(parent),
                    "generated parent {} must be outside the representable range",
                    parent
                );
            }
        }
        Ok(())
    })
    .expect("entity and modifier id charter");
}

// ============================================================================
// H-IR-SERDE: bounded finite serialization, not a whole-IR validator
// ============================================================================

/// Field-level observation of one entity, captured before encoding.
///
/// Integer identity, enum identity, optional presence and selected float bit
/// patterns are recorded separately from the encoding, so the round-trip
/// assertions do not reuse the encoder as their own oracle.
#[derive(Debug, Clone, PartialEq)]
struct EntityObservation {
    entity_id: u64,
    role: ExtrusionRole,
    region_key: RegionKey,
    topo_order: u32,
    tool_index: u32,
    path_role: ExtrusionRole,
    speed_factor_bits: u32,
    path_tool_index: Option<u32>,
    order_lock: Option<u64>,
    point_count: usize,
    first_point_bits: [u32; 5],
}

/// Field-level observation of one layer's selected serializable slice.
#[derive(Debug, Clone, PartialEq)]
struct LayerObservation {
    global_layer_index: u32,
    z_bits: u32,
    entities: Vec<EntityObservation>,
    travels: Vec<(u64, [Option<u32>; 4])>,
    speed_profiles: Vec<(u64, Vec<u32>)>,
    support_rows: Vec<(u64, String, String, Vec<String>)>,
}

fn option_bits(value: Option<f32>) -> Option<u32> {
    value.map(f32::to_bits)
}

fn observe_layers(layers: &[LayerCollectionIR]) -> Vec<LayerObservation> {
    layers
        .iter()
        .map(|layer| LayerObservation {
            global_layer_index: layer.global_layer_index,
            z_bits: layer.z.to_bits(),
            entities: layer
                .ordered_entities
                .iter()
                .map(|entity| {
                    let first = entity.path.points.first().copied().unwrap_or_default();
                    EntityObservation {
                        entity_id: entity.entity_id,
                        role: entity.role.clone(),
                        region_key: entity.region_key.clone(),
                        topo_order: entity.topo_order,
                        tool_index: entity.tool_index,
                        path_role: entity.path.role.clone(),
                        speed_factor_bits: entity.path.speed_factor.to_bits(),
                        path_tool_index: entity.path.tool_index,
                        order_lock: entity.path.order_lock,
                        point_count: entity.path.points.len(),
                        first_point_bits: [
                            first.x.to_bits(),
                            first.y.to_bits(),
                            first.z.to_bits(),
                            first.width.to_bits(),
                            first.flow_factor.to_bits(),
                        ],
                    }
                })
                .collect(),
            travels: layer
                .travel_moves
                .iter()
                .map(|travel| {
                    (
                        travel.entity_id,
                        [
                            option_bits(travel.x),
                            option_bits(travel.y),
                            option_bits(travel.z),
                            option_bits(travel.f),
                        ],
                    )
                })
                .collect(),
            speed_profiles: layer
                .speed_profiles
                .iter()
                .map(|profile| {
                    (
                        profile.entity_id,
                        profile.factors.iter().map(|factor| factor.to_bits()).collect(),
                    )
                })
                .collect(),
            support_rows: layer
                .support_entity_identities
                .iter()
                .map(|identity| {
                    (
                        identity.entity_id,
                        identity.family_id.clone(),
                        identity.body_id.clone(),
                        identity.demand_ids.clone(),
                    )
                })
                .collect(),
        })
        .collect()
}

fn serde_cases(
    domain: EntityDomain,
    max_travels: usize,
) -> Result<BoxedStrategy<(Vec<LayerCollectionIR>, Vec<ExPolygon>)>, DomainError> {
    let entity_count = domain.max_entities;
    let layer_count = domain.max_layers;
    let choices = proptest::collection::vec(
        (
            proptest::collection::vec(any::<u8>(), 0..=entity_count),
            proptest::collection::vec(any::<u64>(), 0..=entity_count),
            proptest::collection::vec(any::<u8>(), 0..=max_travels),
        ),
        0..=layer_count,
    );
    // Integer-grid geometry is the exact-`Point2` half of the selected slice:
    // no float ever participates in its identity.
    let geometry = (
        proptest::collection::vec((-64_i64..=64, -64_i64..=64), 0..=8),
        proptest::collection::vec(
            proptest::collection::vec((-64_i64..=64, -64_i64..=64), 0..=4),
            0..=2,
        ),
    )
        .prop_map(|(contour, holes)| property_oracles::footprint(&contour, &holes));
    Ok((entity_layers(domain)?, choices, geometry)
        .prop_map(move |(layers, choices, footprint)| {
            let mut layers = layers;
            for (index, layer) in layers.iter_mut().enumerate() {
                let (tools, locks, travels) = choices.get(index).cloned().unwrap_or_default();
                for (entity_index, entity) in layer.ordered_entities.iter_mut().enumerate() {
                    let selector = tools.get(entity_index).copied().unwrap_or(0);
                    entity.path.tool_index = (selector % 2 == 1).then_some(u32::from(selector));
                    entity.path.order_lock = locks
                        .get(entity_index)
                        .filter(|lock| *lock % 2 == 0)
                        .copied();
                    entity.tool_index = u32::try_from(entity_index % 3).expect("small selector");
                }
                let anchor_ids: Vec<u64> = layer
                    .ordered_entities
                    .iter()
                    .map(|entity| entity.entity_id)
                    .collect();
                if !anchor_ids.is_empty() {
                    layer.travel_moves = travels
                        .iter()
                        .enumerate()
                        .map(|(travel_index, seed)| {
                            let dyadic = |offset: u8| {
                                f32::from(i16::from(seed.wrapping_add(offset)) % 64) / 8.0
                            };
                            // exhaustive: pins every travel field for the round trip
                            TravelMove {
                                entity_id: anchor_ids[(travel_index * 7) % anchor_ids.len()],
                                x: (seed % 2 == 0).then(|| dyadic(1)),
                                y: (seed % 3 == 0).then(|| dyadic(5)),
                                z: (seed % 5 == 0).then(|| dyadic(9)),
                                f: (seed % 7 == 0).then(|| dyadic(13)),
                            }
                        })
                        .collect();
                }
                // Side tables stay keyed to real entity ids and bounded by the
                // entity's own point count, so a shrink cannot strand a row.
                layer.speed_profiles = layer
                    .ordered_entities
                    .iter()
                    .enumerate()
                    .filter(|(entity_index, entity)| {
                        entity_index % 2 == 0 && !entity.path.points.is_empty()
                    })
                    .map(|(_, entity)| EntitySpeedProfile {
                        entity_id: entity.entity_id,
                        factors: entity.path.points.iter().map(|point| point.width).collect(),
                    })
                    .collect();
                layer.support_entity_identities = layer
                    .ordered_entities
                    .iter()
                    .enumerate()
                    .filter(|(entity_index, _)| entity_index % 2 == 1)
                    .map(|(entity_index, entity)| SupportEntityIdentity {
                        entity_id: entity.entity_id,
                        family_id: format!("family-{entity_index}"),
                        body_id: format!("body-{entity_index}"),
                        demand_ids: vec![format!("demand-{entity_index}")],
                    })
                    .collect();
            }
            (layers, footprint)
        })
        .boxed())
}

#[test]
fn proptest_ir_serialization_and_defaults() {
    let run = PropertyRun::new("H-IR-SERDE", "proptest_ir_serialization_and_defaults")
        .expect("use the registry-backed property command");
    let max_entities = usize::try_from(run.bound("max_entities").expect("entity bound"))
        .expect("entity bound must fit usize");
    let max_travels = usize::try_from(run.bound("max_travels").expect("travel bound"))
        .expect("travel bound must fit usize");
    let max_points = usize::try_from(run.bound("max_points").expect("point bound"))
        .expect("point bound must fit usize");
    let max_layers = usize::try_from(run.bound("max_layers").expect("layer bound"))
        .expect("layer bound must fit usize");
    let domain = EntityDomain {
        roles: vec![
            ExtrusionRole::OuterWall,
            ExtrusionRole::InnerWall,
            ExtrusionRole::SparseInfill,
            ExtrusionRole::SupportMaterial,
        ],
        max_entities,
        max_points,
        max_layers,
    };
    let cases = serde_cases(domain, max_travels).expect("bounded finite layer domain");
    run.check(&cases, |(layers, footprints), _guard| {
        let before = observe_layers(&layers);

        // test-quality: the encoder/decoder pair (postcard::to_allocvec /
        // postcard::from_bytes and serde_json::to_string / from_str) is the
        // surface under test; correctness of the round trip is judged against
        // `observe_layers`, which reads fields directly and never calls either
        // codec.
        let postcard_bytes = postcard::to_allocvec(&layers).expect("layers are Postcard-encodable");
        let from_postcard: Vec<LayerCollectionIR> =
            postcard::from_bytes(&postcard_bytes).expect("Postcard bytes decode");
        prop_assert_eq!(
            &from_postcard,
            &layers,
            "Postcard round trip must preserve the generated finite layers"
        );
        prop_assert_eq!(
            observe_layers(&from_postcard),
            before.clone(),
            "Postcard must preserve ids, roles, keys, float bits and side tables"
        );

        let json = serde_json::to_string(&layers).expect("layers are JSON-encodable");
        let from_json: Vec<LayerCollectionIR> =
            serde_json::from_str(&json).expect("finite JSON decodes");
        prop_assert_eq!(
            &from_json,
            &layers,
            "JSON round trip must preserve the generated finite layers"
        );
        prop_assert_eq!(
            observe_layers(&from_json),
            before,
            "JSON must preserve ids, roles, keys, float bits and side tables"
        );

        // Exact `Point2` integer identity, both codecs, including extrema that
        // a float round trip would destroy.
        let mut exact = footprints.clone();
        exact.push(ExPolygon {
            contour: slicer_ir::Polygon {
                points: vec![
                    Point2 {
                        x: i64::MIN,
                        y: i64::MAX,
                    },
                    Point2 { x: 0, y: 0 },
                ],
            },
            holes: Vec::new(),
        });
        let bytes = postcard::to_allocvec(&exact).expect("integer geometry is Postcard-encodable");
        let decoded: Vec<ExPolygon> = postcard::from_bytes(&bytes).expect("integer bytes decode");
        prop_assert_eq!(&decoded, &exact, "Postcard must keep integer coordinates exact");
        let json = serde_json::to_string(&exact).expect("integer geometry is JSON-encodable");
        let decoded: Vec<ExPolygon> = serde_json::from_str(&json).expect("integer JSON decodes");
        prop_assert_eq!(&decoded, &exact, "JSON must keep integer coordinates exact");
        Ok(())
    })
    .expect("IR serialization and defaults charter");
}


// ============================================================================
// H-CONFIG-INTERN: independent equality/hash and interner model
// ============================================================================

/// One generated configuration seed. The variant tag keeps the two labelled
/// modes apart: `Finite` builds compatible structural fixtures from a bounded
/// magnitude, `Raw` builds representation-contract fixtures that may carry
/// signed zeros, infinities or NaN payloads and are never claimed valid.
#[derive(Debug, Clone)]
enum ConfigSeed {
    /// Compatible structural fixture inside the checked magnitude bound.
    Finite {
        layer_height: u8,
        line_height: u8,
        speed: u8,
        percent: Option<(u8, bool)>,
        option: u8,
        sequence: usize,
        text: usize,
        depth: usize,
        breadth: usize,
    },
    /// Raw representation fixture: exceptional bits are deliberate.
    Raw {
        layer_height_bits: u64,
        speed_bits: u32,
        option_bits: Option<u32>,
        sequence: usize,
        depth: usize,
        breadth: usize,
    },
    /// A bounded scalar pushed through the real `apply_cli_key` seam, so the
    /// shared `ScalarConfigDomain` helpers feed an actual config field.
    Scalar(ConfigValue),
    /// A bounded holder string pushed through `apply_cli_key`.
    Holder(String),
    /// The previous configuration rebuilt from its canonical Postcard bytes,
    /// so equality comes from an independent construction path rather than a
    /// clone of the same value.
    Repeat,
}

fn bounded_magnitude(bound: f64, index: u8) -> f64 {
    // Strictly positive quarter-steps, never above the declared magnitude.
    let steps = f64::from(index % 32) + 1.0;
    (bound * steps / 32.0).min(bound)
}

/// Clamp the extension shape so its node count fits the total list budget.
fn bounded_extension_shape(depth: usize, breadth: usize, budget: usize) -> (usize, usize) {
    let mut breadth = breadth.max(1);
    while breadth
        .checked_pow(u32::try_from(depth).unwrap_or(u32::MAX))
        .is_none_or(|nodes| nodes > budget)
    {
        if breadth == 1 {
            break;
        }
        breadth -= 1;
    }
    (depth, breadth)
}

fn config_from_seed(
    seed: &ConfigSeed,
    previous: Option<&ResolvedConfig>,
    max_abs_value: f64,
    max_list_len: usize,
    max_list_depth: usize,
) -> ResolvedConfig {
    match seed {
        ConfigSeed::Finite {
            layer_height,
            line_height,
            speed,
            percent,
            option,
            sequence,
            text,
            depth,
            breadth,
        } => {
            let depth = (*depth).min(max_list_depth);
            let (depth, breadth) = bounded_extension_shape(depth, *breadth, max_list_len);
            let mut config = ResolvedConfig::default();
            config.layer_height = bounded_magnitude(max_abs_value, *layer_height);
            config.first_layer_height = bounded_magnitude(max_abs_value, line_height.wrapping_add(7));
            config.line_width = bounded_magnitude(max_abs_value, line_height.wrapping_add(3));
            config.outer_wall_speed = bounded_magnitude(max_abs_value, *speed) as f32;
            if let Some((value, is_percent)) = percent {
                config = property_oracles::with_float_or_percent(
                    &config,
                    bounded_magnitude(max_abs_value, *value),
                    *is_percent,
                );
            }
            config = property_oracles::with_option(
                &config,
                (*option % 2 == 1).then(|| bounded_magnitude(max_abs_value, *option) as f32),
            );
            let nodes = breadth.saturating_pow(u32::try_from(depth).unwrap_or(u32::MAX));
            let budget = max_list_len.saturating_sub(nodes);
            let seq = sequence % (budget / 2 + 1);
            let txt = text % (budget - seq + 1);
            config = property_oracles::with_sequence(&config, seq);
            config = property_oracles::with_text_sequence(&config, txt);
            property_oracles::with_extension(&config, depth, breadth)
        }
        ConfigSeed::Raw {
            layer_height_bits,
            speed_bits,
            option_bits,
            sequence,
            depth,
            breadth,
        } => {
            let depth = (*depth).min(max_list_depth);
            let (depth, breadth) = bounded_extension_shape(depth, *breadth, max_list_len);
            let mut config = ResolvedConfig::default();
            config.layer_height = f64::from_bits(*layer_height_bits);
            config.outer_wall_speed = f32::from_bits(*speed_bits);
            config = property_oracles::with_option(&config, option_bits.map(f32::from_bits));
            let nodes = breadth.saturating_pow(u32::try_from(depth).unwrap_or(u32::MAX));
            let budget = max_list_len.saturating_sub(nodes);
            config = property_oracles::with_sequence(&config, sequence % (budget + 1));
            property_oracles::with_extension(&config, depth, breadth)
        }
        ConfigSeed::Scalar(value) => {
            let mut config = ResolvedConfig::default();
            config.layer_height = bounded_magnitude(max_abs_value, 3);
            let applied = config
                .apply_cli_key("wall_count", value)
                .expect("the bounded scalar domain keeps wall_count inside u32");
            assert!(applied, "wall_count must be a bound CLI key");
            config
        }
        ConfigSeed::Holder(text) => {
            let mut config = ResolvedConfig::default();
            config.layer_height = bounded_magnitude(max_abs_value, 5);
            let applied = config
                .apply_cli_key("top_fill_holder", &ConfigValue::String(text.clone()))
                .expect("a bounded holder string is accepted");
            assert!(applied, "top_fill_holder must be a bound CLI key");
            config
        }
        ConfigSeed::Repeat => {
            let previous = previous.cloned().unwrap_or_default();
            let bytes = property_oracles::representation_key(&previous);
            postcard::from_bytes(&bytes).expect("canonical config bytes decode")
        }
    }
}

fn config_cases(
    max_configs: usize,
    max_abs_value: f64,
    max_list_len: usize,
    max_list_depth: usize,
) -> BoxedStrategy<Vec<ResolvedConfig>> {
    let finite = (
        any::<u8>(),
        any::<u8>(),
        any::<u8>(),
        proptest::option::of((any::<u8>(), any::<bool>())),
        any::<u8>(),
        0_usize..=max_list_len,
        0_usize..=max_list_len,
        0_usize..=max_list_depth.max(1),
        1_usize..=4,
    )
        .prop_map(
            |(layer_height, line_height, speed, percent, option, sequence, text, depth, breadth)| {
                ConfigSeed::Finite {
                    layer_height,
                    line_height,
                    speed,
                    percent,
                    option,
                    sequence,
                    text,
                    depth,
                    breadth,
                }
            },
        );
    let raw = (
        any::<u64>(),
        any::<u32>(),
        proptest::option::of(any::<u32>()),
        0_usize..=max_list_len,
        0_usize..=max_list_depth.max(1),
        1_usize..=4,
    )
        .prop_map(
            |(layer_height_bits, speed_bits, option_bits, sequence, depth, breadth)| ConfigSeed::Raw {
                layer_height_bits,
                speed_bits,
                option_bits,
                sequence,
                depth,
                breadth,
            },
        );
    proptest::collection::vec(
        prop_oneof![
            4 => finite,
            3 => raw,
            2 => Just(ConfigSeed::Repeat),
            2 => scalar_config_values(ScalarConfigDomain::Int { min: 1, max: 8 })
                .expect("bounded integer scalar domain")
                .prop_map(ConfigSeed::Scalar),
            1 => scalar_config_values(ScalarConfigDomain::StringChoices(vec![
                "rectilinear-infill".to_string(),
                "concentric-infill".to_string(),
            ]))
            .expect("bounded string-choice domain")
            .prop_map(|value| match value {
                ConfigValue::String(text) => ConfigSeed::Holder(text),
                other => ConfigSeed::Holder(format!("unexpected-{other:?}")),
            }),
            1 => nonfinite_f64().prop_map(|value| ConfigSeed::Raw {
                layer_height_bits: value.to_bits(),
                speed_bits: 0x7fc0_0001,
                option_bits: Some(0x7f80_0001),
                sequence: 0,
                depth: 0,
                breadth: 1,
            }),
        ],
        0..=max_configs,
    )
    .prop_map(move |seeds| {
        let mut configs: Vec<ResolvedConfig> = Vec::with_capacity(seeds.len());
        for seed in &seeds {
            let config = config_from_seed(
                seed,
                configs.last(),
                max_abs_value,
                max_list_len,
                max_list_depth,
            );
            configs.push(config);
        }
        configs
    })
    .boxed()
}

#[test]
fn proptest_config_equality_hash_and_interning() {
    let run = PropertyRun::new(
        "H-CONFIG-INTERN",
        "proptest_config_equality_hash_and_interning",
    )
    .expect("use the registry-backed property command");
    let max_configs = usize::try_from(run.bound("max_configs").expect("config bound"))
        .expect("config bound must fit usize");
    let max_list_len = usize::try_from(run.bound("max_list_len").expect("list bound"))
        .expect("list bound must fit usize");
    let max_list_depth = usize::try_from(run.bound("max_list_depth").expect("depth bound"))
        .expect("depth bound must fit usize");
    let max_abs_value = run.bound("max_abs_config_value").expect("magnitude bound") as f64;
    assert!(
        max_abs_value > 0.0 && max_abs_value <= 16_777_216.0,
        "max_abs_config_value must be positive and at most 2^24, got {max_abs_value}"
    );

    // --- mandatory representation witnesses --------------------------------
    let default = ResolvedConfig::default();
    let positive_zero = property_oracles::with_f64_field(&default, 0.0);
    let negative_zero = property_oracles::with_f64_field(&default, -0.0);
    assert!(
        !property_oracles::struct_eq(&positive_zero, &negative_zero),
        "bitwise config equality keeps the zero sign distinct"
    );
    assert_ne!(
        property_oracles::representation_key(&positive_zero),
        property_oracles::representation_key(&negative_zero),
        "Postcard must keep the zero sign distinct"
    );
    let nan_one = property_oracles::with_f64_field(&default, f64::from_bits(0x7ff8_0000_0000_0001));
    let nan_two = property_oracles::with_f64_field(&default, f64::from_bits(0x7ff8_0000_0000_0002));
    assert!(!property_oracles::struct_eq(&nan_one, &nan_two));
    assert_ne!(
        property_oracles::representation_key(&nan_one),
        property_oracles::representation_key(&nan_two),
        "distinct NaN payloads must stay distinct in the representation"
    );
    // Equal values, independently constructed, agree on equality and hash.
    let rebuilt: ResolvedConfig =
        postcard::from_bytes(&property_oracles::representation_key(&nan_one))
            .expect("canonical config bytes decode");
    assert!(property_oracles::struct_eq(&nan_one, &rebuilt));
    assert_eq!(
        property_oracles::hash_of(&nan_one),
        property_oracles::hash_of(&rebuilt),
        "equal values must hash equally"
    );
    // A float/percent variant change is a real equality change, not a
    // magnitude change.
    let percent = property_oracles::with_float_or_percent(&default, 50.0, true);
    let absolute = property_oracles::with_float_or_percent(&default, 50.0, false);
    assert!(!property_oracles::struct_eq(&percent, &absolute));
    // Real interner behaviour, including the default seed.
    let mut map = RegionMapIR::default();
    assert_eq!(map.intern_config(ResolvedConfig::default()), ConfigId(0));
    let nondefault = property_oracles::with_f32_field(&default, 37.5);
    let id = map.intern_config(nondefault.clone());
    assert_eq!(id, ConfigId(1), "a nondefault config joins the pool at rank 1");
    assert_eq!(map.intern_config(nondefault.clone()), id, "interning is idempotent");
    assert_eq!(map.configs.len(), 2, "no duplicate pool entry");
    let key = RegionKey {
        global_layer_index: 0,
        object_id: "config-intern-witness".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    };
    map.entries.insert(
        key.clone(),
        RegionPlan {
            config: id,
            ..Default::default()
        },
    );
    assert_eq!(
        property_oracles::representation_key(map.config_for(&key)),
        property_oracles::representation_key(&nondefault),
        "a keyed plan must read its nondefault config, not index zero"
    );

    // --- sampled cases -----------------------------------------------------
    let cases = config_cases(max_configs, max_abs_value, max_list_len, max_list_depth);
    run.check(&cases, |configs, _guard| {
        let mut map = RegionMapIR::default();
        let ids: Vec<ConfigId> = configs
            .iter()
            .map(|config| map.intern_config(config.clone()))
            .collect();
        let (expected_ids, expected_pool) = property_oracles::pool_model(&default, &configs);

        prop_assert_eq!(
            &ids,
            &expected_ids,
            "interned ids must equal the first-occurrence representation model"
        );
        prop_assert_eq!(
            map.configs.len(),
            expected_pool.len(),
            "pool growth must match the model"
        );
        for (index, config) in configs.iter().enumerate() {
            prop_assert_eq!(
                property_oracles::representation_key(map.config_for_raw(ids[index])),
                property_oracles::representation_key(config),
                "config_for_raw must return the interned input at rank {}",
                index
            );
        }
        // Equality oracle and the hash implication, over every pair.
        for left in 0..configs.len() {
            for right in 0..configs.len() {
                let actual = property_oracles::struct_eq(&configs[left], &configs[right]);
                let by_representation = property_oracles::representation_key(&configs[left])
                    == property_oracles::representation_key(&configs[right]);
                prop_assert_eq!(
                    actual,
                    by_representation,
                    "equality must agree with canonical representation at ({}, {})",
                    left,
                    right
                );
                if actual {
                    prop_assert_eq!(
                        property_oracles::hash_of(&configs[left]),
                        property_oracles::hash_of(&configs[right]),
                        "equal values must hash equally at ({}, {})",
                        left,
                        right
                    );
                }
            }
        }
        // A keyed plan reads the intended nondefault config, never index zero.
        let default_key = property_oracles::representation_key(&ResolvedConfig::default());
        if let Some((index, config)) = configs
            .iter()
            .enumerate()
            .find(|(_, config)| property_oracles::representation_key(config) != default_key)
        {
            let key = RegionKey {
                global_layer_index: u32::try_from(index).expect("bounded index"),
                object_id: format!("plan-{index}"),
                region_id: 1,
                variant_chain: Vec::new(),
            };
            map.entries.insert(
                key.clone(),
                RegionPlan {
                    config: ids[index],
                    ..Default::default()
                },
            );
            prop_assert_eq!(
                property_oracles::representation_key(map.config_for(&key)),
                property_oracles::representation_key(config),
                "config_for must return the intended nondefault config"
            );
        }
        Ok(())
    })
    .expect("config equality, hash and interning charter");
}

// ============================================================================
// H-CHAINS: independent bounded Cartesian model
// ============================================================================

fn paint_value_cases() -> impl Strategy<Value = PaintValue> {
    prop_oneof![
        any::<bool>().prop_map(PaintValue::Flag),
        (0_u32..=3).prop_map(PaintValue::ToolIndex),
        "[a-z]{1,4}".prop_map(PaintValue::Custom),
    ]
}

fn chain_cases(
    max_semantics: usize,
    max_values: usize,
    max_text_bytes: usize,
) -> BoxedStrategy<(HashMap<String, Vec<PaintValue>>, Vec<String>)> {
    let name_width = max_text_bytes.clamp(2, 8);
    proptest::collection::vec(
        (
            0_u8..=2,
            proptest::collection::vec(paint_value_cases(), 0..=max_values),
        ),
        0..=max_semantics,
    )
    .prop_map(move |specs| {
        let mut variants: HashMap<String, Vec<PaintValue>> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for (index, (mode, values)) in specs.into_iter().enumerate() {
            // Unique, bounded axis names, so the canonical order is a real
            // ordering rather than an artefact of map iteration.
            let name: String = format!("s{index}").chars().take(name_width).collect();
            order.push(name.clone());
            match mode {
                0 => {}
                1 => {
                    variants.insert(name, Vec::new());
                }
                _ => {
                    variants.insert(name, values);
                }
            }
        }
        // Unrelated keys are never enumerated.
        variants.insert(
            "unrelated".chars().take(name_width).collect(),
            vec![PaintValue::Flag(true)],
        );
        (variants, order)
    })
    .boxed()
}

#[test]
fn proptest_canonical_chain_product() {
    let run = PropertyRun::new("H-CHAINS", "proptest_canonical_chain_product")
        .expect("use the registry-backed property command");
    let max_semantics = usize::try_from(run.bound("max_semantics").expect("semantic bound"))
        .expect("semantic bound must fit usize");
    let max_values =
        usize::try_from(run.bound("max_values_per_semantic").expect("value bound"))
            .expect("value bound must fit usize");
    let max_chains = run.bound("max_chains").expect("chain bound") as u128;
    let max_text_bytes = usize::try_from(run.bound("max_text_bytes").expect("text bound"))
        .expect("text bound must fit usize");

    // --- mandatory witnesses ----------------------------------------------
    let empty: HashMap<String, Vec<PaintValue>> = HashMap::new();
    assert_eq!(
        enumerate_canonical_chains(&empty, &[]),
        vec![Vec::new()],
        "no axes must still yield exactly the empty chain"
    );
    let unsorted = vec![
        PaintValue::ToolIndex(2),
        PaintValue::Flag(false),
        PaintValue::ToolIndex(0),
    ];
    let mut single: HashMap<String, Vec<PaintValue>> = HashMap::new();
    single.insert("a".to_string(), unsorted.clone());
    assert_eq!(
        enumerate_canonical_chains(&single, &["a".to_string()]),
        vec![
            Vec::new(),
            vec![("a".to_string(), PaintValue::ToolIndex(2))],
            vec![("a".to_string(), PaintValue::Flag(false))],
            vec![("a".to_string(), PaintValue::ToolIndex(0))],
        ],
        "the empty chain comes first and caller value order is preserved verbatim"
    );
    // Absent and empty axes contribute a factor of one.
    let mut degenerates: HashMap<String, Vec<PaintValue>> = HashMap::new();
    degenerates.insert("empty".to_string(), Vec::new());
    degenerates.insert("unrelated".to_string(), vec![PaintValue::Flag(true)]);
    assert_eq!(
        enumerate_canonical_chains(&degenerates, &["absent".to_string(), "empty".to_string()]),
        vec![Vec::new()],
        "absent and empty axes must both behave as radix one, and extra keys are ignored"
    );
    // Duplicate values keep their multiplicity instead of being deduplicated.
    let mut duplicated: HashMap<String, Vec<PaintValue>> = HashMap::new();
    duplicated.insert(
        "a".to_string(),
        vec![PaintValue::Flag(true), PaintValue::Flag(true)],
    );
    let duplicate_output = enumerate_canonical_chains(&duplicated, &["a".to_string()]);
    assert_eq!(duplicate_output.len(), 3);
    assert_eq!(
        duplicate_output
            .iter()
            .filter(|chain| chain.len() == 1)
            .count(),
        2,
        "two identical values must produce two chains, not one"
    );
    // Impossible Cartesian requests are refused before allocation.
    let wide: BTreeMap<String, Vec<PaintValue>> = (0..64)
        .map(|index| (format!("a{index}"), vec![PaintValue::Flag(true)]))
        .collect();
    let wide_order: Vec<String> = wide.keys().cloned().collect();
    assert!(
        property_oracles::chain_product_size(&wide, &wide_order, max_chains).is_err(),
        "2^64 chains must be refused against a bounded max_chains"
    );

    // --- sampled cases -----------------------------------------------------
    let cases = chain_cases(max_semantics, max_values, max_text_bytes);
    run.check(&cases, |(variants, order), _guard| {
        let btree: BTreeMap<String, Vec<PaintValue>> = variants
            .iter()
            .map(|(name, values)| (name.clone(), values.clone()))
            .collect();
        let size = match property_oracles::chain_product_size(&btree, &order, max_chains) {
            Ok(size) => size,
            Err(error) => {
                prop_assert!(false, "generator exceeded max_chains: {}", error);
                unreachable!("prop_assert returns on failure")
            }
        };
        let actual = enumerate_canonical_chains(&variants, &order);
        prop_assert_eq!(
            actual.len() as u128,
            size,
            "cardinality must equal the checked mixed-radix product"
        );
        let expected = property_oracles::chain_product_model(&btree, &order);
        prop_assert_eq!(
            &actual,
            &expected,
            "the ordered Cartesian model must match the helper exactly"
        );
        if let Some(first) = actual.first() {
            prop_assert!(
                first.is_empty(),
                "the empty chain must be the first enumerated element"
            );
        }
        for chain in &actual {
            let mut previous: Option<&String> = None;
            for (axis, _) in chain {
                if let Some(previous) = previous {
                    prop_assert!(
                        order.iter().position(|name| name == previous)
                            < order.iter().position(|name| name == axis),
                        "each chain must follow the canonical axis order"
                    );
                }
                previous = Some(axis);
            }
        }
        Ok(())
    })
    .expect("canonical chain product charter");
}
