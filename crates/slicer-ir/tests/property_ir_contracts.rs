//! Deterministic controls for the `slicer-ir` property charters.
//!
//! Every test here runs under a plain
//! `cargo test -p slicer-ir --features property_testing --test property_ir_contracts`.
//! Nothing in this file reads `PNP_PROPERTY_CONTEXT`, builds a `PropertyRun`, or
//! consults the property registry: these are hand-computed witnesses that prove
//! the shared reference models in `tests/common/property_oracles.rs` can reject
//! the faults the randomized properties are meant to catch.
//!
//! The models are only worth using if they fail on deliberately wrong
//! implementations, so each control names the mutation it must detect.

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::panic::catch_unwind;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestRunner};

use slicer_ir::region_split_registry::enumerate_canonical_chains;
use slicer_ir::{
    is_modifier_namespace_id, mm_to_units, modifier_base_region_id, modifier_sub_region_id,
    modifier_sub_region_id_fits, units_to_mm, validate_travel_anchors, ConfigId,
    ConfigResolutionError, ConfigValue, ExtrusionRole, LayerCollectionIR, LayerEntityIdGen,
    PaintValue, Point2, PrintEntity, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    TravelMove, MODIFIER_FOOTPRINT_REGION_ID, MODIFIER_VARIANT_REGION_ID_STRIDE, UNITS_PER_MM,
};
use slicer_sdk::coords::{mm_to_units as sdk_mm_to_units, SCALING_FACTOR};
use slicer_sdk::test_support::fixtures::print_entity_base;
use slicer_sdk::test_support::property_testing::domains::{
    anchor_cases, entity_layers, finite_f32, finite_f64, rounding_boundary_mm, AnchorDomain,
    DomainError, EntityDomain,
};
use slicer_sdk::test_support::property_testing::oracles::first_missing_anchor;

#[path = "common/property_oracles.rs"]
#[allow(dead_code)]
mod property_oracles;

use property_oracles::{
    allocation_model, chain_product_model, chain_product_size, check_mm_case, check_mm_case_with,
    check_mm_units_envelope, check_monotonic_pair, check_units_case, decode_model,
    defective_drop_last_product, defective_duplicate_product,
    defective_factor_100, defective_omit_empty_chain, defective_sorted_values,
    defective_swapped_axis_order, defective_truncating, expected_payload_bounds, footprint,
    half_unit_witnesses, hash_of, layer_with_side_table_only_anchor, mm_bound,
    mm_to_units_allowed_error, payload_fits_model, pool_model, representable_parent_limit,
    representation_key, struct_eq, units_bound, with_extension, with_f32_field, with_f64_field,
    with_float_or_percent, with_option, with_sequence, with_text_sequence, CoordProfile,
};

/// The real validator's documented panic text when minting an unsupported
/// modifier parent (`modifier_sub_region_id`,
/// `crates/slicer-ir/src/slice_ir.rs`).
const MODIFIER_PARENT_ASSERTION: &str =
    "modifier sub-region id payload exceeds its namespace or reserved sentinel";

// ============================================================================
// Shared helpers
// ============================================================================

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn point_hash(point: &Point2) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    point.hash(&mut hasher);
    hasher.finish()
}

/// Deliberately wrong reference used as the anchor control's mutation: it
/// unions `ordered_entities` with every unrelated side table that carries an
/// `entity_id`, which is exactly the defect the real validator must not have.
fn side_table_union_first_missing(layer: &LayerCollectionIR) -> Option<u64> {
    let mut ids: std::collections::BTreeSet<u64> = layer
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

/// Membership witness: the oracle and the real validator must agree on the
/// first missing anchor, reordered/repeated/extreme ids included.
fn anchor_membership_witness(entities: &[u64], anchors: &[u64], expected: Option<u64>) {
    let layer = LayerCollectionIR {
        ordered_entities: entities
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
    };
    assert_eq!(
        first_missing_anchor(&layer),
        expected,
        "oracle witness: entities={entities:?}, anchors={anchors:?}"
    );
    match expected {
        Some(missing) => {
            let error =
                validate_travel_anchors(&layer).expect_err("a dangling travel anchor must fail");
            assert!(
                error.contains(&missing.to_string()),
                "error must name the missing id {missing}: {error}"
            );
            assert!(
                error.contains("layer.ordered_entities"),
                "error must name the membership source: {error}"
            );
        }
        None => assert!(
            validate_travel_anchors(&layer).is_ok(),
            "membership must pass: entities={entities:?}, anchors={anchors:?}"
        ),
    }
}

fn fixture_layer() -> LayerCollectionIR {
    LayerCollectionIR {
        ordered_entities: vec![PrintEntity {
            entity_id: 7,
            ..print_entity_base(ExtrusionRole::OuterWall)
        }],
        ..Default::default()
    }
}

fn decode_after_edit(
    layer: &LayerCollectionIR,
    edit: impl FnOnce(&mut serde_json::Value),
) -> Result<LayerCollectionIR, serde_json::Error> {
    let mut value = serde_json::to_value(layer).expect("LayerCollectionIR is JSON-serializable");
    edit(&mut value);
    serde_json::from_value(value)
}

fn assert_config_value_bits(decoded: &ConfigValue, expected: &ConfigValue) {
    match (decoded, expected) {
        (ConfigValue::Float(actual), ConfigValue::Float(want)) => {
            assert_eq!(actual.to_bits(), want.to_bits())
        }
        (ConfigValue::Percent(actual), ConfigValue::Percent(want)) => {
            assert_eq!(actual.to_bits(), want.to_bits())
        }
        (
            ConfigValue::FloatOrPercent {
                value: actual,
                is_percent: actual_percent,
            },
            ConfigValue::FloatOrPercent {
                value: want,
                is_percent: want_percent,
            },
        ) => {
            assert_eq!(actual.to_bits(), want.to_bits());
            assert_eq!(actual_percent, want_percent);
        }
        (actual, want) => panic!("unexpected ConfigValue variant: {actual:?} vs {want:?}"),
    }
}

struct ChainCase {
    map: HashMap<String, Vec<PaintValue>>,
    ordered: BTreeMap<String, Vec<PaintValue>>,
    order: Vec<String>,
}

fn chain_case(variants: &[(&str, Vec<PaintValue>)], order: &[&str]) -> ChainCase {
    ChainCase {
        map: variants
            .iter()
            .map(|(axis, values)| ((*axis).to_owned(), values.clone()))
            .collect(),
        ordered: variants
            .iter()
            .map(|(axis, values)| ((*axis).to_owned(), values.clone()))
            .collect(),
        order: order.iter().map(|axis| (*axis).to_owned()).collect(),
    }
}

fn runner() -> TestRunner {
    TestRunner::new(Config {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x6f72_6163_6c65_7301),
        failure_persistence: None,
        ..Config::default()
    })
}

/// Walk every generated value plus a bounded shrink chain, asserting the
/// domain's own promise on each produced value.
fn visit_shrinks<S: Strategy>(strategy: &S, cases: usize, mut check: impl FnMut(&S::Value)) {
    let mut runner = runner();
    for _ in 0..cases {
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("strategy builds a tree");
        check(&tree.current());
        for _ in 0..16 {
            let changed = tree.simplify();
            check(&tree.current());
            if !changed {
                break;
            }
        }
    }
}

// ============================================================================
// 1. H-COORD controls
// ============================================================================

#[test]
fn ir_coord_oracle_and_bound_controls() {
    // --- checked profile admission -----------------------------------------
    let hundred = CoordProfile::new(100.0, 1_000_000).expect("1 m / 1e6 units is a safe profile");
    let quarter = CoordProfile::new(25.0, 250_000).expect("25 mm / 250k units is a safe profile");
    for profile in [hundred, quarter] {
        let allowed = mm_to_units_allowed_error(&profile);
        assert!(allowed >= 0.5, "the derived envelope starts at 1/2 unit");
        assert!(
            allowed <= mm_bound(&profile) as f64,
            "the integer bound must dominate the derived envelope"
        );
        assert!(units_bound(&profile) >= 0.0);
    }

    // Unsafe profiles are rejected, never truncated. Each entry names the
    // inequality it violates.
    for (max_abs_mm, max_abs_units) in [
        (2_000.0, 1_000_000), // 10_000 * M > 2^24
        (1.0, 16_777_217),    // U > 2^24
        (200.0, 2_000_000),   // min(U, 10_000 * M) = 2_000_000 > 1_000_000
        (f64::INFINITY, 1_000),
        (f64::NAN, 1_000),
        (0.0, 1_000),
        (-1.0, 1_000),
        (1.0, 0),
        (1.0, -1),
    ] {
        assert!(
            CoordProfile::new(max_abs_mm, max_abs_units).is_err(),
            "profile ({max_abs_mm}, {max_abs_units}) must be rejected"
        );
    }
    // The `min(...)` rejection above is specific: 200 mm with a 1e6-unit cap
    // is still admitted, so the rejection came from the shared-boundary cap.
    assert!(CoordProfile::new(200.0, 1_000_000).is_ok());

    // --- derived envelopes on the deterministic witness grid ---------------
    let mut witnesses: Vec<f32> = vec![
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        hundred.max_abs_mm as f32,
        -(hundred.max_abs_mm as f32),
    ];
    for (value, _) in half_unit_witnesses() {
        witnesses.push(value);
    }
    // Rounding-boundary neighbours around `k + 0.5` units, including the
    // profile end.
    for k in [0_i64, 1, 2, 312, 999, 999_999, -1, -312, -999_999] {
        let boundary = ((k as f64 + 0.5) / 10_000.0) as f32;
        witnesses.extend([boundary.next_down(), boundary, boundary.next_up()]);
    }
    for value in &witnesses {
        if let Err(error) = check_mm_case(&hundred, *value) {
            panic!("mm witness {value:?} failed the derived envelope: {error}");
        }
    }
    for (value, derived) in half_unit_witnesses() {
        assert_eq!(
            mm_to_units(value),
            derived,
            "the half-unit witness {value:?} must match the derived half-away value"
        );
    }

    // --- derived unit envelope --------------------------------------------
    for n in [
        0_i64, 1, -1, 312, 313, -313, 999_999, -999_999, 1_000_000, -1_000_000,
    ] {
        if let Err(error) = check_units_case(&hundred, n) {
            panic!("unit witness {n} failed the derived envelope: {error}");
        }
    }
    for n in [0_i64, 250_000, -250_000] {
        assert!(check_units_case(&quarter, n).is_ok());
    }
    assert!(check_units_case(&quarter, 250_001).is_err());
    assert!(check_mm_case(&quarter, 25.001).is_err());

    // --- monotonicity on ordered pairs -------------------------------------
    let ordered_inputs = [
        -100.0f32,
        -0.031_250_004,
        -0.031_25,
        -f32::from_bits(1),
        -0.0,
        0.0,
        f32::from_bits(1),
        0.000_1,
        0.031_25,
        0.031_250_004,
        1.0,
        99.999_99,
        100.0,
    ];
    for window in ordered_inputs.windows(2) {
        if let Err(error) = check_monotonic_pair(&hundred, window[0], window[1]) {
            panic!(
                "monotonicity failed for ({}, {}): {error}",
                window[0], window[1]
            );
        }
    }
    assert!(
        check_monotonic_pair(&hundred, 1.0, 0.5).is_err(),
        "a reversed pair must be rejected as an invalid domain request"
    );

    // --- SDK wrapper / Point2 agreement is consistency only ----------------
    assert_eq!(sdk_mm_to_units(0.031_25), 313);
    assert_eq!(mm_to_units(0.031_25), 313);
    assert_eq!(
        Point2::from_mm(0.031_25, -0.031_25),
        Point2 { x: 313, y: -313 }
    );

    // --- exact integer identity, never through a float ---------------------
    let extreme = Point2 {
        x: i64::MIN,
        y: i64::MAX,
    };
    let bytes = postcard::to_allocvec(&extreme).expect("Point2 is Postcard-serializable");
    let decoded: Point2 = postcard::from_bytes(&bytes).expect("Point2 integer bytes decode");
    assert_eq!(decoded, extreme);
    assert_eq!(point_hash(&decoded), point_hash(&extreme));
    assert_ne!(
        extreme,
        Point2 {
            x: i64::MIN + 1,
            y: i64::MAX
        }
    );

    // --- mutation controls -------------------------------------------------
    let half = 1.0f32 / 32.0;
    // The envelope alone is tie-blind: truncating 312.5 to 312 is exactly half
    // a unit away and therefore inside `1/2 + e_mul`.
    let truncated = defective_truncating(half);
    assert_eq!(truncated, 312);
    assert!(
        check_mm_units_envelope(&hundred, half, truncated).is_ok(),
        "the derived envelope is expected to admit the truncating mutant"
    );
    // The same callable checker `check_mm_case` delegates to rejects both
    // mutants: the factor-of-100 scaling by the envelope, truncation by the
    // derived exact half-away tie rule.
    assert!(
        check_mm_case_with(&hundred, half, defective_factor_100).is_err(),
        "the factor-of-100 mutant must be rejected"
    );
    assert!(
        check_mm_case_with(&hundred, half, defective_truncating).is_err(),
        "the truncating mutant must be rejected at the exact half-unit input"
    );

    // --- the universal exact-inverse claim is false ------------------------
    // The claim is falsified by a concrete pair, not by a constant-true
    // scaffolding predicate: no helper asserts an always-true value here.
    assert_eq!(mm_to_units(half), 313);
    assert_ne!(
        f64::from(units_to_mm(313)),
        0.031_25_f64,
        "1/32 mm -> 313 units -> 0.0313 mm is not an exact inverse"
    );
    let exact_inverse_holds =
        |value: f32| f64::from(units_to_mm(mm_to_units(value))) == f64::from(value);
    assert!(
        !exact_inverse_holds(half),
        "the false exact-roundtrip claim must be falsified at the 1/32 mm half case"
    );
    let subnormal = f32::from_bits(1);
    assert_eq!(mm_to_units(subnormal), 0);
    assert_ne!(
        f64::from(units_to_mm(mm_to_units(subnormal))),
        f64::from(subnormal)
    );

    assert_eq!(UNITS_PER_MM, 10_000.0);
    assert_eq!(SCALING_FACTOR, 10_000);
}

// ============================================================================
// 2. H-IDS controls
// ============================================================================

#[test]
fn ir_id_parent_and_sentinel_rejections() {
    let stride = MODIFIER_VARIANT_REGION_ID_STRIDE;

    // --- allocation model --------------------------------------------------
    let allocations = |count: usize| -> Vec<u64> {
        let ids = LayerEntityIdGen::new();
        (0..count).map(|_| ids.next()).collect()
    };
    assert_eq!(allocations(0), Vec::<u64>::new());
    assert_eq!(allocations(1), vec![1]);
    for count in 1..=64_u64 {
        assert_eq!(
            allocations(count as usize),
            allocation_model(count),
            "{count} fresh allocations must match 1..={count}"
        );
    }
    assert_eq!(
        allocations(16),
        allocations(16),
        "repeated fresh generators must agree"
    );
    assert!(
        !allocation_model(64).contains(&0),
        "the reserved zero sentinel is never allocated"
    );

    // --- independently derived representable-parent limit ------------------
    let limit = representable_parent_limit();
    assert!(modifier_sub_region_id_fits(limit));
    assert!(!modifier_sub_region_id_fits(limit + 1));
    assert!(!modifier_sub_region_id_fits(u64::MAX));
    assert!(payload_fits_model(limit));
    assert!(!payload_fits_model(limit + 1));
    assert!(!payload_fits_model(u64::MAX));
    for parent in [
        0_u64,
        1,
        2,
        stride - 1,
        stride,
        stride + 1,
        limit - 1,
        limit,
    ] {
        assert_eq!(
            payload_fits_model(parent),
            modifier_sub_region_id_fits(parent),
            "model and system must agree on supported parent {parent}"
        );
    }
    assert!(
        !payload_fits_model(limit + 1) && !modifier_sub_region_id_fits(limit + 1),
        "the representable boundary is exactly P"
    );

    // --- unsupported parents assert with the documented message ------------
    let sample_footprint = footprint(&[(0, 0), (10, 0), (0, 10)], &[]);
    for parent in [limit + 1, u64::MAX] {
        let outcome = catch_unwind(|| modifier_sub_region_id(parent, "obj", &sample_footprint));
        let error = outcome.expect_err("minting must assert on an unsupported parent");
        let message = panic_message(error);
        assert!(
            message.contains(MODIFIER_PARENT_ASSERTION),
            "unexpected panic message for parent {parent}: {message}"
        );
    }

    // --- supported parents: namespace, payload, decode ---------------------
    let footprint_witnesses = [
        footprint(&[], &[]),
        footprint(&[(0, 0), (10, 0), (0, 10)], &[]),
        footprint(
            &[(0, 0), (20, 0), (20, 20), (0, 20)],
            &[vec![(5, 5), (6, 5), (5, 6)]],
        ),
    ];
    for parent in [
        0_u64,
        1,
        2,
        stride - 1,
        stride,
        stride + 1,
        limit - 1,
        limit,
    ] {
        assert!(modifier_sub_region_id_fits(parent));
        for footprint_geo in &footprint_witnesses {
            let id = modifier_sub_region_id(parent, "obj", footprint_geo);
            assert_eq!(id >> 63, 1, "minted ids carry the namespace marker");
            assert_ne!(id, MODIFIER_FOOTPRINT_REGION_ID);
            assert!(is_modifier_namespace_id(id));
            assert_eq!(modifier_base_region_id(id), Some(parent));
            // test-quality: determinism pin on `modifier_sub_region_id`; the
            // namespace/payload/decode assertions are the independent checks.
            assert_eq!(
                id,
                modifier_sub_region_id(parent, "obj", footprint_geo),
                "identical inputs must mint identical ids"
            );
            let (quotient, remainder) =
                decode_model(id).expect("a high-bit-marked non-sentinel id decodes");
            assert_eq!(quotient, parent);
            assert!(
                (1..stride).contains(&remainder),
                "payload remainder {remainder} must lie in 1..{stride}"
            );
            let payload = id & !(1u64 << 63);
            let (low, high) = expected_payload_bounds(parent, remainder);
            assert!(
                low <= payload && payload <= high,
                "payload {payload} outside the model interval [{low}, {high}]"
            );
        }
    }

    // Distinct parents cannot share an id for the same object/footprint. The
    // converse is deliberately not asserted: different object/footprint inputs
    // hash into a bounded stride and may collide, so no universal injectivity
    // or unequal-hash claim is made anywhere here.
    let same_inputs = footprint(&[(0, 0), (10, 0), (0, 10)], &[]);
    assert_ne!(
        modifier_sub_region_id(1, "obj", &same_inputs),
        modifier_sub_region_id(2, "obj", &same_inputs)
    );

    // --- sentinels and namespace membership --------------------------------
    assert_eq!(modifier_base_region_id(MODIFIER_FOOTPRINT_REGION_ID), None);
    assert!(!is_modifier_namespace_id(MODIFIER_FOOTPRINT_REGION_ID));
    assert_eq!(decode_model(MODIFIER_FOOTPRINT_REGION_ID), None);
    for ordinary in [0_u64, 1, 12_345, u64::MAX >> 1] {
        assert!(!is_modifier_namespace_id(ordinary));
        assert_eq!(modifier_base_region_id(ordinary), None);
        assert_eq!(decode_model(ordinary), None);
    }
    // Namespace membership is not provenance validation: an arbitrary
    // bit-marked id decodes even though it was never minted.
    let never_minted = (1u64 << 63) | (7 * stride + 3);
    assert!(is_modifier_namespace_id(never_minted));
    assert_eq!(modifier_base_region_id(never_minted), Some(7));
    assert_eq!(decode_model(never_minted), Some((7, 3)));
}

// ============================================================================
// 3. H-IR anchor control
// ============================================================================

#[test]
fn ir_anchor_oracle_rejects_side_table_membership() {
    let travel_id = 9_u64;
    let layer = layer_with_side_table_only_anchor(&[1, 2, 3], travel_id);
    assert!(layer
        .ordered_entities
        .iter()
        .all(|entity| entity.entity_id != travel_id));
    assert_eq!(layer.speed_profiles.len(), 1);
    assert_eq!(layer.speed_profiles[0].entity_id, travel_id);

    let error = validate_travel_anchors(&layer)
        .expect_err("a travel anchor present only in a side table must not be accepted");
    assert!(
        error.contains(&travel_id.to_string()),
        "error must name the missing travel id: {error}"
    );
    assert!(
        error.contains("layer.ordered_entities"),
        "error must name the membership source: {error}"
    );
    assert_eq!(first_missing_anchor(&layer), Some(travel_id));

    // The controlled mutation: a "side-table union" reference accepts the
    // anchor because `speed_profiles` mentions it. The real validator and the
    // independent membership oracle both disagree, so this mutation is caught.
    assert_eq!(side_table_union_first_missing(&layer), None);
    assert_ne!(
        side_table_union_first_missing(&layer),
        first_missing_anchor(&layer)
    );

    // Ordered, repeated and extreme-ID witnesses through the prerequisite
    // oracle, to show the anchor control does not depend on the side table.
    anchor_membership_witness(&[], &[], None);
    anchor_membership_witness(&[7], &[], None);
    anchor_membership_witness(&[7], &[7], None);
    anchor_membership_witness(&[], &[9], Some(9));
    anchor_membership_witness(&[7], &[7, 99, 2], Some(99)); // travel order, not sorted ids
    anchor_membership_witness(&[7], &[7, 7, 7], None); // repeated references
    anchor_membership_witness(&[9, 2, 7], &[7, 9, 2], None); // reordered entities
    anchor_membership_witness(&[7, 7], &[7, 7], None); // duplicate entities
    anchor_membership_witness(&[0, u64::MAX], &[u64::MAX, 0, u64::MAX], None);
    anchor_membership_witness(&[u64::MAX], &[0], Some(0));
    anchor_membership_witness(&[0], &[u64::MAX], Some(u64::MAX));
}

// ============================================================================
// 4. H-IR-SERDE controls
// ============================================================================

// test-quality: encode/decode roundtrip pair — postcard::to_allocvec /
// postcard::from_bytes over raw ConfigValue and scalar values; the assertions
// compare exact IEEE bit patterns, so the pair is what is under test and
// native NaN equality is never used.
#[test]
fn ir_serde_defaults_and_nonfinite_policy() {
    let layer = fixture_layer();
    let baseline: LayerCollectionIR =
        serde_json::from_value(serde_json::to_value(&layer).expect("fixture serializes"))
            .expect("the full fixture JSON decodes");
    assert_eq!(baseline, layer);

    // --- each legacy-defaulted field is asserted independently -------------
    let decoded = decode_after_edit(&layer, |value| {
        let removed = value["ordered_entities"][0]
            .as_object_mut()
            .expect("entity is a JSON object")
            .remove("tool_index");
        assert!(
            removed.is_some(),
            "the fixture must actually serialize PrintEntity.tool_index"
        );
    })
    .expect("omitting PrintEntity.tool_index must decode");
    assert_eq!(decoded.ordered_entities[0].tool_index, 0);

    let decoded = decode_after_edit(&layer, |value| {
        let removed = value["ordered_entities"][0]["path"]
            .as_object_mut()
            .expect("path is a JSON object")
            .remove("tool_index");
        assert!(
            removed.is_some(),
            "the fixture must actually serialize ExtrusionPath3D.tool_index"
        );
    })
    .expect("omitting ExtrusionPath3D.tool_index must decode");
    assert_eq!(decoded.ordered_entities[0].path.tool_index, None);

    let decoded = decode_after_edit(&layer, |value| {
        let removed = value["ordered_entities"][0]["path"]
            .as_object_mut()
            .expect("path is a JSON object")
            .remove("order_lock");
        assert!(
            removed.is_some(),
            "the fixture must actually serialize ExtrusionPath3D.order_lock"
        );
    })
    .expect("omitting ExtrusionPath3D.order_lock must decode");
    assert_eq!(decoded.ordered_entities[0].path.order_lock, None);

    let decoded = decode_after_edit(&layer, |value| {
        let removed = value
            .as_object_mut()
            .expect("layer is a JSON object")
            .remove("speed_profiles");
        assert!(removed.is_some());
    })
    .expect("omitting LayerCollectionIR.speed_profiles must decode");
    assert!(decoded.speed_profiles.is_empty());

    let decoded = decode_after_edit(&layer, |value| {
        let removed = value
            .as_object_mut()
            .expect("layer is a JSON object")
            .remove("support_entity_identities");
        assert!(removed.is_some());
    })
    .expect("omitting LayerCollectionIR.support_entity_identities must decode");
    assert!(decoded.support_entity_identities.is_empty());

    // --- required fields are not defaulted ---------------------------------
    let missing_id = decode_after_edit(&layer, |value| {
        let removed = value["ordered_entities"][0]
            .as_object_mut()
            .expect("entity is a JSON object")
            .remove("entity_id");
        assert!(removed.is_some());
    });
    assert!(
        missing_id.is_err(),
        "omitting the required PrintEntity.entity_id must reject"
    );

    let wrong_id_type = decode_after_edit(&layer, |value| {
        value["ordered_entities"][0]["entity_id"] = serde_json::Value::String("seven".to_owned());
    });
    assert!(
        wrong_id_type.is_err(),
        "a string PrintEntity.entity_id must reject"
    );

    // --- JSON null in a required numeric field is rejected -----------------
    let null_x = decode_after_edit(&layer, |value| {
        value["ordered_entities"][0]["path"]["points"][0]["x"] = serde_json::Value::Null;
    });
    assert!(
        null_x.is_err(),
        "a JSON null in the required Point3WithWidth.x must reject"
    );
    let null_width = decode_after_edit(&layer, |value| {
        value["ordered_entities"][0]["path"]["points"][0]["width"] = serde_json::Value::Null;
    });
    assert!(
        null_width.is_err(),
        "a JSON null in the required Point3WithWidth.width must reject"
    );

    // --- raw exceptional Postcard bits (representation only) ---------------
    // These are NOT production-valid config or model inputs: they exist to
    // pin the bit-preserving policy of the Postcard boundary.
    for bits in [
        0.0_f64.to_bits(),
        (-0.0_f64).to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_0000_0000_0001,
        0x7ff8_0000_0000_0002,
    ] {
        let value = f64::from_bits(bits);
        for candidate in [
            ConfigValue::Float(value),
            ConfigValue::Percent(value),
            ConfigValue::FloatOrPercent {
                value,
                is_percent: true,
            },
            ConfigValue::FloatOrPercent {
                value,
                is_percent: false,
            },
        ] {
            let bytes =
                postcard::to_allocvec(&candidate).expect("ConfigValue is Postcard-serializable");
            let decoded: ConfigValue =
                postcard::from_bytes(&bytes).expect("raw ConfigValue bits decode");
            assert_config_value_bits(&decoded, &candidate);
        }
        let bytes = postcard::to_allocvec(&value).expect("raw f64 is Postcard-serializable");
        let decoded: f64 = postcard::from_bytes(&bytes).expect("raw f64 bits decode");
        assert_eq!(decoded.to_bits(), value.to_bits());
    }
    for bits in [
        0.0_f32.to_bits(),
        (-0.0_f32).to_bits(),
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        0x7fc0_0001,
        0x7fc0_0002,
    ] {
        let value = f32::from_bits(bits);
        let bytes = postcard::to_allocvec(&value).expect("raw f32 is Postcard-serializable");
        let decoded: f32 = postcard::from_bytes(&bytes).expect("raw f32 bits decode");
        assert_eq!(decoded.to_bits(), value.to_bits());
    }
}

// ============================================================================
// 5. H-CONFIG-INTERN controls
// ============================================================================

#[test]
fn ir_config_oracle_counterexamples() {
    let base = ResolvedConfig::default();
    let nan_one = f64::from_bits(0x7ff8_0000_0000_0001);
    let nan_two = f64::from_bits(0x7ff8_0000_0000_0002);

    // --- premise validation: representation keys distinguish these pairs ---
    // Each pair must be unequal under the real `==` *and* under the Postcard
    // key, otherwise the `actual_eq == representation_eq` oracle is vacuous.
    let mut sequence_content = with_sequence(&base, 4);
    // Index 0 is `0.0` in the generator, so changing it to `1.0` changes content
    // without changing length (index 2 already holds `1.0`, which would be a
    // no-op mutation).
    sequence_content.bed_shape[0] = 1.0;
    let premise_pairs = [
        (with_f64_field(&base, 0.0), with_f64_field(&base, -0.0)),
        (
            with_f64_field(&base, nan_one),
            with_f64_field(&base, nan_two),
        ),
        (with_option(&base, None), with_option(&base, Some(0.5))),
        (with_sequence(&base, 4), with_sequence(&base, 5)),
        (with_sequence(&base, 4), sequence_content),
        (with_text_sequence(&base, 1), with_text_sequence(&base, 2)),
        (with_extension(&base, 1, 2), with_extension(&base, 2, 2)),
    ];
    for (index, (left, right)) in premise_pairs.iter().enumerate() {
        assert_ne!(
            representation_key(left),
            representation_key(right),
            "the Postcard representation key must distinguish counterexample pair {index}"
        );
        assert!(
            !struct_eq(left, right),
            "the real equality must also distinguish counterexample pair {index}"
        );
    }

    // --- equality oracle on a deterministic corpus -------------------------
    let corpus = vec![
        base.clone(),
        with_f64_field(&base, 0.2),
        with_f64_field(&base, 0.3),
        with_f64_field(&base, -0.0),
        with_f32_field(&base, 50.0),
        with_f32_field(&base, 51.0),
        with_f32_field(&base, -0.0),
        with_float_or_percent(&base, 100.0, true),
        with_float_or_percent(&base, 100.0, false),
        with_option(&base, None),
        with_option(&base, Some(1.5)),
        with_sequence(&base, 2),
        with_text_sequence(&base, 1),
        with_extension(&base, 1, 2),
        with_extension(&base, 2, 2),
        with_f64_field(&base, nan_one),
        with_f64_field(&base, nan_two),
    ];
    for left in &corpus {
        for right in &corpus {
            assert_eq!(
                struct_eq(left, right),
                representation_key(left) == representation_key(right),
                "actual equality and representation equality must agree"
            );
            if struct_eq(left, right) {
                assert_eq!(
                    hash_of(left),
                    hash_of(right),
                    "equal configs must hash equally (the implication is one-way)"
                );
            }
        }
    }

    // --- first-occurrence interning model ----------------------------------
    let first = with_f64_field(&base, 0.3);
    // Independently constructed equal value: JSON round-trip preserves the
    // exact f64 bits for finite values.
    let first_from_json: ResolvedConfig =
        serde_json::from_str(&serde_json::to_string(&first).expect("fixture serializes"))
            .expect("fixture JSON decodes");
    assert_eq!(first, first_from_json);
    let second = with_f32_field(&base, 51.0);
    let third = with_extension(&base, 1, 2);
    let inputs = vec![
        first.clone(),
        second.clone(),
        first_from_json,
        third.clone(),
        second.clone(),
        first.clone(),
    ];

    // --- nested-list budget counts total elements, not leaves ---------------
    // Depth 2, breadth 2 builds 2 + 4 = 6 list elements (the intermediate list
    // plus its two children), not 4; a budget of 4 must clamp the breadth.
    assert_eq!(
        property_oracles::extension_node_count(2, 2),
        Some(6),
        "a depth-2 breadth-2 extension tree allocates 2 + 4 list elements"
    );
    assert_eq!(
        property_oracles::extension_node_count(3, 2),
        Some(14),
        "depth 3 adds the 8-element level"
    );
    let (clamped_depth, clamped_breadth) = property_oracles::clamped_extension_shape(2, 2, 4);
    assert_eq!(clamped_depth, 2, "the depth bound is separate from the budget");
    assert!(
        property_oracles::extension_node_count(clamped_depth, clamped_breadth) <= Some(4),
        "a budget of 4 must clamp the shape to at most 4 total list elements, got {:?}",
        property_oracles::extension_node_count(clamped_depth, clamped_breadth)
    );
    // Concrete regression: a depth-2 breadth-2 request against a total budget of
    // 4 previously produced 6 elements. The constructed fixture must stay inside
    // the budget for both sequence lengths and the extension tree together.
    let mut budgeted = ResolvedConfig::default();
    let (depth, breadth) = property_oracles::clamped_extension_shape(2, 2, 4);
    budgeted = property_oracles::with_sequence(&budgeted, 0);
    budgeted = property_oracles::with_text_sequence(&budgeted, 0);
    budgeted = property_oracles::with_extension(&budgeted, depth, breadth);
    assert!(
        property_oracles::config_list_elements(&budgeted) <= 4,
        "the clamped fixture must respect the total budget, got {} elements",
        property_oracles::config_list_elements(&budgeted)
    );

    let (model_ids, model_pool) = pool_model(&base, &inputs);

    let mut map = RegionMapIR::default();
    let actual_ids: Vec<ConfigId> = inputs
        .iter()
        .cloned()
        .map(|config| map.intern_config(config))
        .collect();
    assert_eq!(actual_ids, model_ids);
    assert_eq!(map.configs.len(), model_pool.len());
    for (index, id) in model_ids.iter().enumerate() {
        assert_eq!(
            map.config_for_raw(*id),
            &model_pool[id.0 as usize],
            "input {index} must resolve to its modelled pool entry"
        );
        assert_eq!(
            &inputs[index],
            map.config_for_raw(*id),
            "interning input {index} must resolve to an equal config"
        );
    }
    assert_eq!(
        actual_ids[0], actual_ids[2],
        "equal configs intern to one id"
    );
    assert_ne!(actual_ids[0], actual_ids[1]);
    assert_eq!(actual_ids[0], ConfigId(1));

    // Re-interning is idempotent.
    let again: Vec<ConfigId> = inputs
        .iter()
        .cloned()
        .map(|config| map.intern_config(config))
        .collect();
    assert_eq!(again, actual_ids);
    assert_eq!(map.configs.len(), model_pool.len());

    // A real keyed plan reads the intended non-default config, not index 0.
    let non_default = actual_ids[1];
    assert_ne!(non_default, ConfigId(0));
    let key = RegionKey {
        global_layer_index: 3,
        object_id: "obj-1".to_owned(),
        region_id: 11,
        variant_chain: Vec::new(),
    };
    map.entries.insert(
        key.clone(),
        RegionPlan {
            config: non_default,
            ..Default::default()
        },
    );
    assert_eq!(map.config_for(&key), &second);
    assert_ne!(map.config_for(&key), map.config_for_raw(ConfigId(0)));
    assert_eq!(map.config_for_raw(ConfigId(0)), &ResolvedConfig::default());

    // --- defect controls ---------------------------------------------------
    // (a) omitted-field comparator: compares only a subset of the fields.
    fn subset_eq(left: &ResolvedConfig, right: &ResolvedConfig) -> bool {
        left.layer_height.to_bits() == right.layer_height.to_bits()
            && left.wall_count == right.wall_count
    }
    let speed_50 = with_f32_field(&base, 50.0);
    let speed_60 = with_f32_field(&base, 60.0);
    assert!(
        subset_eq(&speed_50, &speed_60),
        "the omitted-field comparator is expected to equate configs that differ only in a field it skips"
    );
    assert!(!struct_eq(&speed_50, &speed_60));
    assert_ne!(
        representation_key(&speed_50),
        representation_key(&speed_60),
        "the representation oracle must not share the omitted-field defect"
    );

    // (b) native float equality wrongly equates signed zero.
    let positive_zero = with_f64_field(&base, 0.0);
    let negative_zero = with_f64_field(&base, -0.0);
    assert!(positive_zero.layer_height == negative_zero.layer_height);
    assert!(!struct_eq(&positive_zero, &negative_zero));
    assert_ne!(
        representation_key(&positive_zero),
        representation_key(&negative_zero)
    );

    // (c) default-pool read: returning index 0 always must fail the keyed read.
    assert_ne!(map.config_for_raw(ConfigId(0)), &second);
    assert_ne!(map.config_for_raw(ConfigId(0)), map.config_for(&key));
}

// ============================================================================
// 6. H-CHAINS controls
// ============================================================================

#[test]
fn ir_chain_oracle_order_and_empty_controls() {
    let empty_chain: Vec<(String, PaintValue)> = Vec::new();

    // --- (1) no axes: exactly one empty chain ------------------------------
    let empty = chain_case(&[], &[]);
    assert_eq!(
        enumerate_canonical_chains(&empty.map, &empty.order),
        vec![empty_chain.clone()]
    );
    assert_eq!(
        chain_product_size(&empty.ordered, &empty.order, 16).expect("one empty chain"),
        1
    );
    assert_eq!(
        chain_product_model(&empty.ordered, &empty.order),
        vec![empty_chain.clone()]
    );
    assert_ne!(
        defective_omit_empty_chain(&empty.ordered, &empty.order),
        vec![empty_chain.clone()],
        "the omit-empty-chain mutant must be detectable on the minimal witness"
    );

    // --- (2) single axis: omit-first, then the supplied (unsorted) order ---
    let values = vec![
        PaintValue::ToolIndex(2),
        PaintValue::ToolIndex(0),
        PaintValue::Custom("a".to_owned()),
    ];
    let single = chain_case(&[("material", values.clone())], &["material"]);
    let actual = enumerate_canonical_chains(&single.map, &single.order);
    let expected = vec![
        empty_chain.clone(),
        vec![("material".to_owned(), values[0].clone())],
        vec![("material".to_owned(), values[1].clone())],
        vec![("material".to_owned(), values[2].clone())],
    ];
    assert_eq!(actual, expected);
    assert_eq!(
        actual[1][0].1,
        PaintValue::ToolIndex(2),
        "caller value order must be preserved, not sorted"
    );
    assert_eq!(
        chain_product_model(&single.ordered, &single.order),
        expected
    );
    assert_ne!(
        defective_sorted_values(&single.ordered, &single.order),
        expected,
        "the sorted-values mutant must be detectable"
    );

    // --- (3) two axes: hand-computed, then model agreement -----------------
    let wall = vec![PaintValue::ToolIndex(0)];
    let infill = vec![
        PaintValue::Custom("a".to_owned()),
        PaintValue::Custom("b".to_owned()),
    ];
    let two = chain_case(
        &[("wall", wall.clone()), ("infill", infill.clone())],
        &["wall", "infill"],
    );
    let expected_two = vec![
        empty_chain.clone(),
        vec![("infill".to_owned(), infill[0].clone())],
        vec![("infill".to_owned(), infill[1].clone())],
        vec![("wall".to_owned(), wall[0].clone())],
        vec![
            ("wall".to_owned(), wall[0].clone()),
            ("infill".to_owned(), infill[0].clone()),
        ],
        vec![
            ("wall".to_owned(), wall[0].clone()),
            ("infill".to_owned(), infill[1].clone()),
        ],
    ];
    let actual_two = enumerate_canonical_chains(&two.map, &two.order);
    assert_eq!(actual_two, expected_two);
    assert_eq!(actual_two.len(), 6);
    assert_eq!(chain_product_model(&two.ordered, &two.order), expected_two);
    assert_eq!(
        chain_product_size(&two.ordered, &two.order, 6).expect("six chains"),
        6
    );
    assert!(chain_product_size(&two.ordered, &two.order, 5).is_err());
    assert_ne!(
        defective_swapped_axis_order(&two.ordered, &two.order),
        expected_two,
        "the swapped-axis mutant must be detectable"
    );
    assert_ne!(
        defective_drop_last_product(&two.ordered, &two.order),
        expected_two,
        "the missing-product mutant must be detectable"
    );
    assert_ne!(
        defective_duplicate_product(&two.ordered, &two.order),
        expected_two,
        "the duplicate-product mutant must be detectable"
    );

    // --- (4) three axes: full ordered comparison against the model ---------
    let three = chain_case(
        &[
            ("wall", vec![PaintValue::ToolIndex(0)]),
            (
                "infill",
                vec![PaintValue::Flag(true), PaintValue::Custom("z".to_owned())],
            ),
            ("material", vec![PaintValue::ToolIndex(1)]),
        ],
        &["wall", "infill", "material"],
    );
    let actual_three = enumerate_canonical_chains(&three.map, &three.order);
    assert_eq!(
        actual_three,
        chain_product_model(&three.ordered, &three.order)
    );
    assert_eq!(actual_three.len(), 2 * 3 * 2);
    for chain in &actual_three {
        let positions: Vec<usize> = chain
            .iter()
            .map(|(axis, _)| {
                three
                    .order
                    .iter()
                    .position(|candidate| candidate == axis)
                    .expect("emitted axis must be canonical")
            })
            .collect();
        assert!(
            positions.windows(2).all(|window| window[0] < window[1]),
            "chain axes must follow canonical order: {chain:?}"
        );
    }

    // --- (5) absent key and empty value list are both radix 1 --------------
    let sparse = chain_case(&[("infill", Vec::new())], &["wall", "infill"]);
    assert_eq!(
        enumerate_canonical_chains(&sparse.map, &sparse.order),
        vec![empty_chain.clone()]
    );
    assert_eq!(
        chain_product_size(&sparse.ordered, &sparse.order, 1).expect("two radix-1 axes"),
        1
    );
    let absent = chain_case(
        &[("wall", vec![PaintValue::Flag(false)])],
        &["wall", "infill"],
    );
    assert_eq!(
        enumerate_canonical_chains(&absent.map, &absent.order).len(),
        2,
        "the absent axis contributes radix 1 and the present axis radix 2"
    );

    // --- (6) extra map keys outside canonical order are ignored ------------
    let with_extra = chain_case(
        &[
            ("wall", vec![PaintValue::ToolIndex(0)]),
            (
                "unrelated",
                vec![PaintValue::Flag(false), PaintValue::Flag(true)],
            ),
        ],
        &["wall"],
    );
    let without_extra = chain_case(&[("wall", vec![PaintValue::ToolIndex(0)])], &["wall"]);
    assert_eq!(
        enumerate_canonical_chains(&with_extra.map, &with_extra.order),
        enumerate_canonical_chains(&without_extra.map, &without_extra.order)
    );

    // --- (7) unrelated HashMap insertion order does not change output ------
    let mut forward: HashMap<String, Vec<PaintValue>> = HashMap::new();
    forward.insert("wall".to_owned(), wall.clone());
    forward.insert("infill".to_owned(), infill.clone());
    let mut backward: HashMap<String, Vec<PaintValue>> = HashMap::with_capacity(64);
    backward.insert("infill".to_owned(), infill.clone());
    backward.insert("wall".to_owned(), wall.clone());
    let canonical: Vec<String> = vec!["wall".to_owned(), "infill".to_owned()];
    assert_eq!(
        enumerate_canonical_chains(&forward, &canonical),
        enumerate_canonical_chains(&backward, &canonical)
    );

    // --- (8) duplicate values produce multiplicities, not deduplication ----
    let duplicated = chain_case(
        &[(
            "material",
            vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(1)],
        )],
        &["material"],
    );
    let actual_duplicated = enumerate_canonical_chains(&duplicated.map, &duplicated.order);
    assert_eq!(actual_duplicated.len(), 3);
    assert_eq!(actual_duplicated[1], actual_duplicated[2]);
    assert_eq!(
        actual_duplicated[1],
        vec![("material".to_owned(), PaintValue::ToolIndex(1))]
    );
    assert_eq!(
        chain_product_size(&duplicated.ordered, &duplicated.order, 3).expect("three chains"),
        3
    );
}

// ============================================================================
// 7. Domain rejection and shrink-retention controls
// ============================================================================

#[test]
fn ir_property_domain_rejections() {
    // --- coordinate profiles: overflowing/narrowing bounds are rejected ----
    for (max_abs_mm, max_abs_units) in [
        (2_000.0, 1_000_000),
        (1.0, 16_777_217),
        (200.0, 2_000_000),
        (f64::INFINITY, 1_000),
        (f64::NAN, 1_000),
        (0.0, 1_000),
        (-1.0, 1_000),
        (1.0, 0),
        (1.0, -1),
    ] {
        assert!(
            CoordProfile::new(max_abs_mm, max_abs_units).is_err(),
            "profile ({max_abs_mm}, {max_abs_units}) must be rejected"
        );
    }

    // --- shared rounding-boundary domain -----------------------------------
    for limit in [0_i64, -1, 1_000_001] {
        assert!(
            matches!(
                rounding_boundary_mm(limit),
                Err(DomainError::InvalidBounds(_))
            ),
            "boundary limit {limit} must be rejected"
        );
    }

    // --- entity-layer domain ----------------------------------------------
    let roles = vec![ExtrusionRole::OuterWall];
    assert!(matches!(
        entity_layers(EntityDomain {
            roles: Vec::new(),
            max_entities: 1,
            max_points: 1,
            max_layers: 1,
        }),
        Err(DomainError::InvalidInput(_))
    ));
    for domain in [
        EntityDomain {
            roles: roles.clone(),
            max_entities: 129,
            max_points: 1,
            max_layers: 1,
        },
        EntityDomain {
            roles: roles.clone(),
            max_entities: 1,
            max_points: 129,
            max_layers: 1,
        },
        EntityDomain {
            roles: roles.clone(),
            max_entities: 1,
            max_points: 1,
            max_layers: 129,
        },
        EntityDomain {
            roles: roles.clone(),
            max_entities: 128,
            max_points: 128,
            max_layers: 128,
        },
    ] {
        assert!(
            matches!(
                entity_layers(domain.clone()),
                Err(DomainError::InvalidBounds(_))
            ),
            "entity domain {domain:?} must be rejected"
        );
    }
    assert!(entity_layers(EntityDomain {
        roles: roles.clone(),
        max_entities: 8,
        max_points: 8,
        max_layers: 8,
    })
    .is_ok());

    // --- anchor domain -----------------------------------------------------
    assert!(matches!(
        anchor_cases(AnchorDomain::Dangling, 4, 0),
        Err(DomainError::InvalidBounds(_))
    ));
    assert!(anchor_cases(AnchorDomain::Valid, 4, 0).is_ok());

    // --- impossible Cartesian request is rejected before allocation --------
    let mut wide = BTreeMap::new();
    let mut order = Vec::new();
    for index in 0..64 {
        let axis = format!("axis-{index:02}");
        wide.insert(axis.clone(), vec![PaintValue::Flag(true)]);
        order.push(axis);
    }
    let error = chain_product_size(&wide, &order, 64)
        .expect_err("2^64 chains must be rejected before any allocation");
    assert!(
        error.contains("max_chains"),
        "the rejection must name the requested limit: {error}"
    );

    // --- valid shrinks retain their domain witnesses -----------------------
    let f32_range = -100.0f32..=100.0f32;
    visit_shrinks(
        &finite_f32(f32_range.clone()).expect("finite f32 domain"),
        8,
        |value| {
            assert!(
                value.is_finite() && f32_range.contains(value),
                "shrunk f32 {value:?} escaped its declared range"
            );
        },
    );
    let f64_range = -100.0f64..=100.0f64;
    visit_shrinks(
        &finite_f64(f64_range.clone()).expect("finite f64 domain"),
        8,
        |value| {
            assert!(
                value.is_finite() && f64_range.contains(value),
                "shrunk f64 {value:?} escaped its declared range"
            );
        },
    );
    visit_shrinks(
        &rounding_boundary_mm(1_000).expect("bounded boundary probes"),
        8,
        |value| {
            assert!(
                mm_to_units(*value).unsigned_abs() <= 1_000,
                "boundary probe {value:?} left the declared unit bound"
            );
        },
    );

    let entity_domain = EntityDomain {
        roles: vec![ExtrusionRole::OuterWall, ExtrusionRole::SparseInfill],
        max_entities: 3,
        max_points: 4,
        max_layers: 2,
    };
    visit_shrinks(
        &entity_layers(entity_domain.clone()).expect("bounded entity layers"),
        8,
        |layers| {
            assert!(layers.len() <= entity_domain.max_layers);
            let mut last_id = 0_u64;
            for layer in layers {
                assert!(layer.ordered_entities.len() <= entity_domain.max_entities);
                for entity in &layer.ordered_entities {
                    assert!(entity_domain.roles.contains(&entity.role));
                    assert!(entity_domain.roles.contains(&entity.path.role));
                    assert!(!entity.path.points.is_empty());
                    assert!(entity.path.points.len() <= entity_domain.max_points);
                    assert!(
                        entity.entity_id > last_id,
                        "entity ids stay per-generator monotonic through shrinking"
                    );
                    last_id = entity.entity_id;
                }
            }
        },
    );

    visit_shrinks(
        &anchor_cases(AnchorDomain::Dangling, 4, 3).expect("dangling anchor domain"),
        8,
        |case| {
            assert!(case.layer.travel_moves.len() <= 3);
            assert!(
                first_missing_anchor(&case.layer).is_some(),
                "dangling mode must retain a dangling anchor through shrinking"
            );
            assert!(validate_travel_anchors(&case.layer).is_err());
        },
    );
}

// ============================================================================
// 8. Config type / integer-range rejections
// ============================================================================

#[test]
fn ir_config_type_and_integer_range_rejections() {
    let mut config = ResolvedConfig::default();

    let error = config
        .apply_cli_key("layer_height", &ConfigValue::Bool(true))
        .expect_err("Bool on the f64 layer_height field must fail");
    assert_eq!(
        error,
        ConfigResolutionError::TypeMismatch {
            key: "layer_height".to_owned(),
            expected: "Float",
            actual: "Bool".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "config key 'layer_height': expected Float value, got Bool"
    );

    let negative = config
        .apply_cli_key("wall_count", &ConfigValue::Int(-1))
        .expect_err("a negative wall count must fail");
    assert_eq!(
        negative,
        ConfigResolutionError::OutOfRange {
            key: "wall_count".to_owned(),
            value: -1.0,
            min: Some(0.0),
            max: Some(f64::from(u32::MAX)),
            index: None,
        }
    );

    let too_large = config
        .apply_cli_key("wall_count", &ConfigValue::Int(4_294_967_296))
        .expect_err("an above-u32 wall count must fail");
    assert_eq!(
        too_large,
        ConfigResolutionError::OutOfRange {
            key: "wall_count".to_owned(),
            value: 4_294_967_296.0,
            min: Some(0.0),
            max: Some(f64::from(u32::MAX)),
            index: None,
        }
    );

    // Positive control: the rejections above are not vacuous.
    let applied = config
        .apply_cli_key("wall_count", &ConfigValue::Int(3))
        .expect("a valid wall count must apply");
    assert!(applied);
    assert_eq!(config.wall_count, 3);
}

/// The mode-pinning mechanism behind every `mode_locked` owner strategy.
///
/// `prop_oneof!` unions shrink into earlier alternatives; this probe proves that
/// a `no_shrink` mode selector fed through `prop_flat_map` keeps the variant tag
/// fixed across `simplify()` while the payload still shrinks. It is the mechanism
/// test for the charters' claim that shrinking preserves a case's mode.
#[test]
fn mode_lock_preserves_tags_under_simplification() {
    #[derive(Debug, Clone)]
    enum Tagged {
        A(#[allow(dead_code)] u32),
        B(#[allow(dead_code)] u32),
        C(#[allow(dead_code)] u32),
    }

    fn tag(value: &Tagged) -> u8 {
        match value {
            Tagged::A(_) => 0,
            Tagged::B(_) => 1,
            Tagged::C(_) => 2,
        }
    }

    let strategy = property_oracles::mode_locked(vec![0_u8, 1, 2], |mode| match mode {
        0 => (0_u32..=1000).prop_map(Tagged::A).boxed(),
        1 => (0_u32..=1000).prop_map(Tagged::B).boxed(),
        _ => (0_u32..=1000).prop_map(Tagged::C).boxed(),
    });
    let mut runner = TestRunner::new(Config {
        cases: 8,
        rng_seed: RngSeed::Fixed(0x5eed_c0de),
        failure_persistence: None,
        ..Config::default()
    });
    let mut simplifications = 0_usize;
    let mut observed = [false; 3];
    for _ in 0..8 {
        let mut tree = strategy.new_tree(&mut runner).expect("mode-locked tree");
        let initial = tag(&tree.current());
        observed[usize::from(initial)] = true;
        let mut guard = 0;
        while guard < 64 && tree.simplify() {
            guard += 1;
            simplifications += 1;
            assert_eq!(
                tag(&tree.current()),
                initial,
                "a mode-locked strategy must not change variant while shrinking"
            );
        }
    }
    assert!(
        simplifications > 0,
        "the probe must actually simplify something, otherwise it certifies nothing"
    );
    assert!(
        observed.iter().filter(|seen| **seen).count() >= 2,
        "the probe must observe more than one mode: {observed:?}"
    );
}
