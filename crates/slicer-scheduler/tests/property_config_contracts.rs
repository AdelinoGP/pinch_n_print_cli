//! Deterministic controls for the numeric config-bounds property.
//!
//! `S` in the packet's verification list. These tests are independent of the
//! registry context: they run without `PNP_PROPERTY_CONTEXT` and pin the
//! reference model, the defective mutants it must refute, the domain
//! rejections, and a non-vacuous positive control on the real API.

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestRunner};
use slicer_ir::ConfigValue;
use slicer_scheduler::{BoundsDeclaration, ConfigBoundsIndex, ConfigResolutionError};
use slicer_sdk::test_support::property_testing::domains::{
    scalar_config_values, DomainError, ScalarConfigDomain,
};

#[allow(dead_code)]
#[path = "common/property_config_oracles.rs"]
mod property_config_oracles;

use property_config_oracles::{
    accept_all, bound_values, declaration_sets, derive_probe, describe, ensure_bounded_witness,
    expected_accepts, expected_first_invalid_index, lose_index, mode_locked, off_by_one_bound,
    respects_bound, union_instead_of_intersection, ProbeMode, KEY_ALPHABET, MAX_ABS_BOUND,
    MODULE_ALPHABET, PROBE_MODES,
};

#[test]
fn config_bounds_oracle_counterexamples() {
    // accept_all: a bounded key with a value above its maximum.
    let bounded = vec![declaration("k0", Some(0.0), Some(1.0), "mod_a")];
    let above = ConfigValue::Float(2.0);
    let bounded_index = index_of(&bounded);
    assert!(
        bounded_index.check("k0", &above).is_err(),
        "witness: key k0 = Float(2.0) against declarations {} must be rejected",
        describe(&bounded)
    );
    assert!(
        !expected_accepts(&bounded, "k0", &above),
        "the reference predicate must reject key k0 = Float(2.0) against {}",
        describe(&bounded)
    );
    assert!(
        accept_all(&bounded, "k0", &above),
        "the accept-all mutant accepts key k0 = Float(2.0) against {}; the harness must \
         distinguish it from the real API",
        describe(&bounded)
    );

    // union_instead_of_intersection: two declarations whose union is much wider
    // than the intersection [5, 10]. 3 is below the effective minimum and 50 is
    // above the effective maximum, yet both lie inside the union [0, 100].
    let intersected = vec![
        declaration("k0", Some(0.0), Some(10.0), "mod_a"),
        declaration("k0", Some(5.0), Some(100.0), "mod_b"),
    ];
    let intersected_index = index_of(&intersected);
    for value in [3.0_f64, 50.0] {
        assert!(
            intersected_index
                .check("k0", &ConfigValue::Float(value))
                .is_err(),
            "witness: key k0 = Float({value}) against declarations {} must be rejected",
            describe(&intersected)
        );
        assert!(
            !expected_accepts(&intersected, "k0", &ConfigValue::Float(value)),
            "the reference predicate must reject key k0 = Float({value}) against the \
             intersection [5, 10] of {}; a union-based reference is not the contract",
            describe(&intersected)
        );
        assert!(
            union_instead_of_intersection(&intersected, "k0", &ConfigValue::Float(value)),
            "the union mutant accepts key k0 = Float({value}) against {} although the real API \
             rejects it; intersection [5, 10] is not the union [0, 100]",
            describe(&intersected)
        );
    }
    assert!(
        intersected_index
            .check("k0", &ConfigValue::Float(7.0))
            .is_ok(),
        "7 is inside the intersection [5, 10]"
    );
    // The reference predicate must classify the inside value as accepted too, so
    // substituting a union-based reference is refuted in both directions.
    assert!(
        expected_accepts(&intersected, "k0", &ConfigValue::Float(7.0)),
        "the reference predicate must accept the inside value 7 for {}",
        describe(&intersected)
    );

    // off_by_one_bound: the inclusive minimum and maximum are accepted by the
    // real API but rejected by a strict-comparison mutant.
    let edged = vec![declaration("k0", Some(1.0), Some(3.0), "mod_a")];
    let edged_index = index_of(&edged);
    for value in [1.0_f64, 3.0] {
        assert!(
            edged_index.check("k0", &ConfigValue::Float(value)).is_ok(),
            "witness: key k0 = Float({value}) is an inclusive edge and must be accepted"
        );
        // The reference predicate itself must agree with the inclusive edge, so
        // replacing it with a strict-comparison predicate is refuted here rather
        // than only showing that the mutant disagrees with production.
        assert!(
            expected_accepts(&edged, "k0", &ConfigValue::Float(value)),
            "the reference predicate must accept the inclusive edge Float({value}) for {}",
            describe(&edged)
        );
        assert!(
            !off_by_one_bound(&edged, "k0", &ConfigValue::Float(value)),
            "the off-by-one mutant rejects the inclusive edge Float({value}) for key k0 against \
             declarations {}",
            describe(&edged)
        );
    }

    // lose_index: the real API reports the first offending list position, the
    // mutant always reports None.
    let listed = vec![declaration("k0", Some(0.0), Some(1.0), "mod_a")];
    let listed_index = index_of(&listed);
    let offending = ConfigValue::List(vec![
        ConfigValue::Float(0.5),
        ConfigValue::Float(0.5),
        ConfigValue::Float(9.0),
    ]);
    match listed_index.check("k0", &offending) {
        Err(ConfigResolutionError::OutOfRange {
            index: Some(position),
            ..
        }) => assert_eq!(
            position,
            2,
            "witness: key k0 = {offending:?} against declarations {} reports position 2",
            describe(&listed)
        ),
        other => panic!("expected OutOfRange with index 2, got {other:?}"),
    }
    assert_eq!(
        expected_first_invalid_index(&listed, "k0", &offending),
        Some(2),
        "the reference must locate the first offending position in {offending:?}"
    );
    assert_eq!(
        lose_index(&listed, "k0", &offending),
        None,
        "the lost-index mutant drops position 2 for key k0 = {offending:?} against declarations {}",
        describe(&listed)
    );
}

#[test]
fn config_property_domain_rejections() {
    // A `DomainError` is returned, not a panic: each call below must evaluate
    // to Err instead of aborting the test.
    for (declarations, bound) in [
        (0_usize, 10.0_f64),
        (65, 10.0),
        (4, 0.0),
        (4, -1.0),
        (4, f64::NAN),
        (4, f64::INFINITY),
        (4, f64::NEG_INFINITY),
        (4, MAX_ABS_BOUND * 2.0),
    ] {
        assert_invalid_bounds(declaration_sets(declarations, bound));
    }
    for (bound, list_len) in [
        (10.0_f64, 0_usize),
        (10.0, 65),
        (0.0, 4),
        (f64::NAN, 4),
        (f64::INFINITY, 4),
        (MAX_ABS_BOUND * 2.0, 4),
    ] {
        assert_invalid_bounds(bound_values(bound, list_len));
    }
    // The shared scalar domain keeps rejecting overflowing/narrowing ranges;
    // this property consumes it rather than reimplementing it.
    assert_invalid_bounds(scalar_config_values(ScalarConfigDomain::Float {
        min: f64::INFINITY,
        max: 1.0,
    }));
    assert_invalid_bounds(scalar_config_values(ScalarConfigDomain::Float {
        min: 1.0,
        max: -1.0,
    }));
    assert_invalid_bounds(scalar_config_values(ScalarConfigDomain::Percent {
        min: 2.0,
        max: 1.0,
    }));
    assert_invalid_bounds(scalar_config_values(ScalarConfigDomain::FloatOrPercent {
        min: 0.0,
        max: f64::NAN,
        is_percent: true,
    }));

    // Valid shrinks stay inside the documented bound and keep the produced
    // value families.
    visit_shrinks(
        &bound_values(1000.0, 4).expect("valid value domain"),
        32,
        |value| {
            assert!(
                respects_bound(value, 1000.0, 4),
                "a shrunk value left the domain: {value:?}"
            );
            let ConfigValue::List(elements) = value else {
                return;
            };
            assert!(
                elements.len() <= 4,
                "a shrunk list exceeded its length bound: {elements:?}"
            );
            for element in elements {
                if let ConfigValue::Int(integer) = element {
                    assert_eq!(
                        (*integer as f64) as i64,
                        *integer,
                        "a shrunk Int left the exactly representable integer grid: {integer}"
                    );
                }
                assert!(
                    !matches!(element, ConfigValue::List(_)),
                    "the value domain only produces flat lists: {element:?}"
                );
            }
        },
    );
    visit_shrinks(
        &declaration_sets(4, 1000.0).expect("valid declaration domain"),
        32,
        |declarations| {
            assert!(
                (1..=4).contains(&declarations.len()),
                "a shrunk set left the declaration count bound: {}",
                declarations.len()
            );
            for declaration in declarations {
                assert!(
                    KEY_ALPHABET.contains(&declaration.key.as_str()),
                    "a shrunk set left the key alphabet: {:?}",
                    declaration.key
                );
                assert!(
                    MODULE_ALPHABET.contains(&declaration.module_id.as_str()),
                    "a shrunk set left the module alphabet: {:?}",
                    declaration.module_id
                );
                for bound in [declaration.min, declaration.max].into_iter().flatten() {
                    assert!(
                        bound.is_finite() && bound.abs() <= 1000.0,
                        "a shrunk bound left [-1000, 1000]: {bound:?} in {}",
                        describe(std::slice::from_ref(declaration))
                    );
                }
            }
        },
    );
}

#[test]
fn config_bounds_intersection_positive_control() {
    // Two modules declaring the same key: [0, 10] and [5, 100] intersect to
    // [5, 10]. This is the positive control that keeps the harness non-vacuous:
    // the real API must accept inside the intersection and reject a value that
    // only one declaration would have allowed.
    let bounds = index_of(&[
        declaration("k0", Some(0.0), Some(10.0), "mod_a"),
        declaration("k0", Some(5.0), Some(100.0), "mod_b"),
    ]);
    match bounds.check("k0", &ConfigValue::Float(3.0)) {
        Err(ConfigResolutionError::OutOfRange {
            key,
            value,
            min,
            max,
            index,
        }) => {
            assert_eq!(key, "k0");
            assert_eq!(value.to_bits(), 3.0_f64.to_bits());
            assert_eq!(min.map(f64::to_bits), Some(5.0_f64.to_bits()));
            assert_eq!(max.map(f64::to_bits), Some(10.0_f64.to_bits()));
            assert_eq!(index, None);
        }
        other => panic!("3 is between the two minimums and must be rejected, got {other:?}"),
    }
    for value in [5.0_f64, 7.0, 10.0] {
        assert!(
            bounds.check("k0", &ConfigValue::Float(value)).is_ok(),
            "{value} is inside the intersection [5, 10]"
        );
    }
    assert!(
        bounds.check("k0", &ConfigValue::Float(11.0)).is_err(),
        "11 is above the strictest maximum and must be rejected"
    );
}

fn declaration(
    key: &str,
    min: Option<f64>,
    max: Option<f64>,
    module_id: &str,
) -> BoundsDeclaration {
    BoundsDeclaration {
        key: key.to_string(),
        min,
        max,
        module_id: module_id.to_string(),
    }
}

fn index_of(declarations: &[BoundsDeclaration]) -> ConfigBoundsIndex {
    ConfigBoundsIndex::from_declarations(declarations.iter().cloned())
}

fn assert_invalid_bounds<T>(result: Result<T, DomainError>) {
    match result {
        Err(DomainError::InvalidBounds(_)) => (),
        Err(other) => panic!("expected an InvalidBounds domain error, got {other:?}"),
        Ok(_) => panic!("expected an InvalidBounds domain error, got Ok"),
    }
}

/// Walk `simplify()` a bounded number of steps and observe every produced value.
fn visit_shrinks<S: Strategy>(strategy: &S, cases: usize, mut observe: impl FnMut(&S::Value)) {
    let mut runner = TestRunner::new(Config {
        cases: cases as u32,
        max_local_rejects: 0,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x0f1e_2d3c),
        failure_persistence: None,
        ..Config::default()
    });
    let mut reductions = 0_usize;
    for _ in 0..cases {
        let mut tree = strategy.new_tree(&mut runner).expect("case generation");
        observe(&tree.current());
        for _ in 0..64 {
            let changed = tree.simplify();
            observe(&tree.current());
            if !changed {
                break;
            }
            reductions += 1;
        }
    }
    assert!(
        reductions > 0,
        "the shrink walk must actually simplify at least one generated value"
    );
}

/// The mode-pinning mechanism behind the scheduler value and probe families.
///
/// Proves that a `no_shrink` mode selector fed through `prop_flat_map` keeps the
/// variant tag fixed across `simplify()` while the payload still shrinks.
#[test]
fn mode_lock_preserves_tags_under_simplification() {
    #[derive(Debug, Clone)]
    enum Tagged {
        Scalar(#[allow(dead_code)] f64),
        NonFinite(#[allow(dead_code)] f64),
        List(#[allow(dead_code)] usize),
    }

    fn tag(value: &Tagged) -> u8 {
        match value {
            Tagged::Scalar(_) => 0,
            Tagged::NonFinite(_) => 1,
            Tagged::List(_) => 2,
        }
    }

    let strategy = mode_locked(vec![0_u8, 1, 2], |mode| match mode {
        0 => (-1000.0_f64..=1000.0).prop_map(Tagged::Scalar).boxed(),
        1 => Just(f64::NAN).prop_map(Tagged::NonFinite).boxed(),
        _ => (0_usize..=8).prop_map(Tagged::List).boxed(),
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

/// Every derived probe mode has the verdict and first-invalid index its name
/// claims, on a concrete declaration set.
///
/// This is what makes the shrink-preservation claim falsifiable: the composed
/// strategy re-derives each value from the declaration set, so a mode that names
/// "violating at index 1" must produce exactly that.
#[test]
fn derived_probe_modes_have_their_documented_verdicts() {
    let mut declarations = vec![declaration("k0", Some(0.0), Some(1.0), "mod_a")];
    ensure_bounded_witness(&mut declarations);
    let scalar = ConfigValue::Float(0.5);
    let list = ConfigValue::List(vec![ConfigValue::Float(0.5)]);
    let derive = |mode| derive_probe(mode, &declarations, scalar.clone(), list.clone(), 8);
    let verdict = |mode| {
        let value = derive(mode);
        let key = mode.key_of("k0");
        (expected_accepts(&declarations, &key, &value), expected_first_invalid_index(&declarations, &key, &value))
    };

    for mode in [ProbeMode::EdgeMin, ProbeMode::EdgeMax, ProbeMode::InsideScalar, ProbeMode::InsideList] {
        assert_eq!(verdict(mode), (true, None), "{mode:?} must be accepted inside the witness interval");
    }
    assert_eq!(verdict(ProbeMode::ViolationScalarLow), (false, None), "below the minimum must be rejected");
    assert_eq!(verdict(ProbeMode::ViolationScalarHigh), (false, None), "above the maximum must be rejected");
    assert_eq!(verdict(ProbeMode::ViolationListAt0), (false, Some(0)), "index 0 violation");
    assert_eq!(verdict(ProbeMode::ViolationListAt1), (false, Some(1)), "index 1 violation");
    assert_eq!(verdict(ProbeMode::ViolationListAt2), (false, Some(2)), "index 2 violation");
    assert_eq!(verdict(ProbeMode::NonFiniteScalar), (false, None), "NaN against a bounded key is rejected");
    assert_eq!(verdict(ProbeMode::NonFiniteList), (false, Some(0)), "a NaN list element is rejected at index 0");

    // The general families are independent draws: only their shape is pinned.
    for mode in [ProbeMode::GeneralScalar, ProbeMode::AbsentScalar] {
        let value = derive(mode);
        assert!(!matches!(value, ConfigValue::List(_)), "{mode:?} must stay scalar");
    }
    for mode in [ProbeMode::GeneralList, ProbeMode::AbsentList] {
        let value = derive(mode);
        assert!(matches!(value, ConfigValue::List(_)), "{mode:?} must stay a list");
    }
    // An absent key accepts anything, including a non-finite value.
    assert!(expected_accepts(&declarations, &ProbeMode::AbsentScalar.key_of("k0"), &ConfigValue::Float(f64::NAN)));

    // The declared family of every mode matches the value it derives.
    for mode in PROBE_MODES {
        let value = derive(mode);
        let actual = u8::from(matches!(value, ConfigValue::List(_)));
        assert_eq!(actual, mode.family(), "{mode:?} declared the wrong family");
    }
}
