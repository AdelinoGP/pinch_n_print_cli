//! Registry-backed property for the numeric config-bounds contract.
//!
//! `H-CONFIG-BOUNDS` drives the real public
//! `slicer_scheduler::config_resolution::ConfigBoundsIndex::from_declarations` /
//! `check` surface (and therefore the private `check_value` / `check_scalar`
//! beneath it) against the independent all-declarations model in
//! `tests/common/property_config_oracles.rs`.

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestCaseError, TestRunner};
use slicer_ir::ConfigValue;
use slicer_scheduler::{BoundsDeclaration, ConfigBoundsIndex, ConfigResolutionError};
use slicer_sdk::test_support::property_testing::PropertyRun;

#[allow(dead_code)]
#[path = "common/property_config_oracles.rs"]
mod property_config_oracles;

use property_config_oracles::{
    bound_list_values, bound_scalar_values, declaration_sets, derive_probe, describe,
    expected_accepts, expected_effective, expected_first_invalid_index, present_keys, scalar_bits,
    PROBE_MODES,
};

/// How many deterministic cases the population guard samples before the run.
const PRE_SAMPLE_CASES: usize = 64;

#[test]
fn proptest_config_scalar_bounds() {
    let run = PropertyRun::new("H-CONFIG-BOUNDS", "proptest_config_scalar_bounds")
        .expect("registry context");
    let max_declarations =
        usize::try_from(run.bound("max_declarations").expect("declaration bound"))
            .expect("declaration bound fits usize");
    let max_list_len = usize::try_from(run.bound("max_list_len").expect("list bound"))
        .expect("list bound fits usize");
    let max_abs_bound = exact_bound(run.bound("max_abs_bound").expect("absolute bound"));

    // Mandatory witnesses run on every execution, before any random sampling.
    assert_mandatory_bounds_witnesses();

    let strategy = config_bounds_strategy(max_declarations, max_abs_bound, max_list_len);
    // Audit the composed strategy. The value shape must survive every real
    // `simplify()` call, and for witness-derived probes so must the verdict class
    // and the first-invalid index — the value is re-derived from the shrunk
    // declarations, so those are stable by construction rather than by luck.
    property_config_oracles::assert_mode_preserved(
        &strategy,
        "H-CONFIG-BOUNDS",
        |(decls, key, value)| {
            let family = u8::from(matches!(value, ConfigValue::List(_)));
            if key == property_config_oracles::WITNESS_KEY {
                let class = u8::from(!expected_accepts(decls, key, value));
                let index = expected_first_invalid_index(decls, key, value).map_or(0, |position| {
                    u8::try_from(position.min(2)).expect("bounded") + 1
                });
                family * 16 + class * 8 + index
            } else {
                family * 16
            }
        },
    );
    assert_substantive_population(&strategy);

    run.check(&strategy, |(declarations, key, value), _guard| {
        let index = ConfigBoundsIndex::from_declarations(declarations.iter().cloned());
        let actual = index.check(&key, &value);
        let expected = expected_accepts(&declarations, &key, &value);
        let context = || {
            format!(
                "key {key:?} value {value:?} declarations {}",
                describe(&declarations)
            )
        };
        prop_assert_eq!(
            actual.is_ok(),
            expected,
            "bounds verdict mismatch for {}",
            context()
        );
        match &actual {
            Ok(()) => Ok(()),
            Err(ConfigResolutionError::OutOfRange {
                key: actual_key,
                value: actual_value,
                min,
                max,
                index: actual_index,
            }) => {
                let (expected_min, expected_max) = expected_effective(&declarations, &key)
                    .expect("a rejected key must be present in some declaration");
                prop_assert_eq!(actual_key, &key, "OutOfRange key for {}", context());
                prop_assert_eq!(
                    actual_value.to_bits(),
                    rejected_scalar(&value, *actual_index).to_bits(),
                    "OutOfRange value for {}",
                    context()
                );
                prop_assert_eq!(
                    min.map(f64::to_bits),
                    expected_min.map(f64::to_bits),
                    "OutOfRange min for {}",
                    context()
                );
                prop_assert_eq!(
                    max.map(f64::to_bits),
                    expected_max.map(f64::to_bits),
                    "OutOfRange max for {}",
                    context()
                );
                prop_assert_eq!(
                    *actual_index,
                    expected_first_invalid_index(&declarations, &key, &value),
                    "OutOfRange index for {}",
                    context()
                );
                Ok(())
            }
            Err(other) => Err(TestCaseError::fail(format!(
                "expected OutOfRange, got {other:?} for {}",
                context()
            ))),
        }
    })
    .expect("config bounds contract");
}

/// Narrow a registry `u64` bound to `f64` without silent rounding.
fn exact_bound(raw: u64) -> f64 {
    assert!(
        raw <= 9_007_199_254_740_992,
        "max_abs_bound must be exactly representable as f64, got {raw}"
    );
    raw as f64
}

fn config_bounds_strategy(
    max_declarations: usize,
    max_abs_bound: f64,
    max_list_len: usize,
) -> impl Strategy<Value = (Vec<BoundsDeclaration>, String, ConfigValue)> {
    let declarations =
        declaration_sets(max_declarations, max_abs_bound).expect("declaration domain");
    let modes = proptest::sample::select(PROBE_MODES.to_vec()).no_shrink();
    let scalars = bound_scalar_values(max_abs_bound).expect("scalar value domain");
    let lists = bound_list_values(max_abs_bound, max_list_len).expect("list value domain");
    let picks = 0_usize..=64;
    (declarations, modes, scalars, lists, picks).prop_map(
        move |(mut declarations, mode, general_scalar, general_list, pick)| {
            property_config_oracles::ensure_bounded_witness(&mut declarations);
            let keys = present_keys(&declarations);
            let chosen = if pick == 0 {
                property_config_oracles::ABSENT_KEY.to_string()
            } else {
                keys[(pick - 1) % keys.len()].clone()
            };
            let key = mode.key_of(&chosen);
            let value = derive_probe(
                mode,
                &declarations,
                general_scalar,
                general_list,
                max_list_len,
            );
            (declarations, key, value)
        },
    )
}

/// Scalar magnitude production reports for the rejected position.
fn rejected_scalar(value: &ConfigValue, index: Option<usize>) -> f64 {
    match index {
        Some(position) => match value {
            ConfigValue::List(elements) => scalar_bits(&elements[position])
                .expect("generated list elements are flat numeric values"),
            other => panic!("index {position} reported for a non-list value: {other:?}"),
        },
        None => scalar_bits(value).expect("a rejected value is numeric"),
    }
}

/// Deterministic mandatory witnesses. These run regardless of sampling luck.
fn assert_mandatory_bounds_witnesses() {
    const KEY: &str = "k0";

    // A key named by no declaration is never inspected, whatever the value.
    let absent = index_of(&[declaration(KEY, Some(-1.0), Some(1.0))]);
    assert!(
        absent.check("k1", &ConfigValue::Float(1.0e300)).is_ok(),
        "an absent key must accept a finite value"
    );
    assert!(
        absent.check("k1", &ConfigValue::Float(f64::NAN)).is_ok(),
        "an absent key must accept NaN: the bounds surface is not a universal finite validator"
    );

    // Present but unbounded still runs the finite check.
    let unbounded = index_of(&[declaration(KEY, None, None)]);
    assert!(
        unbounded.check(KEY, &ConfigValue::Float(0.5)).is_ok(),
        "a present but unbounded declaration must accept a finite value"
    );
    assert_out_of_range(
        &unbounded,
        KEY,
        &ConfigValue::Float(f64::NAN),
        f64::NAN,
        (None, None),
        None,
    );
    assert_out_of_range(
        &unbounded,
        KEY,
        &ConfigValue::Float(f64::INFINITY),
        f64::INFINITY,
        (None, None),
        None,
    );

    // One-sided declarations.
    let min_only = index_of(&[declaration(KEY, Some(2.0), None)]);
    assert!(min_only.check(KEY, &ConfigValue::Float(1.0e12)).is_ok());
    assert_out_of_range(
        &min_only,
        KEY,
        &ConfigValue::Float(1.0),
        1.0,
        (Some(2.0), None),
        None,
    );
    let max_only = index_of(&[declaration(KEY, None, Some(2.0))]);
    assert!(max_only.check(KEY, &ConfigValue::Float(-1.0e12)).is_ok());
    assert_out_of_range(
        &max_only,
        KEY,
        &ConfigValue::Float(3.0),
        3.0,
        (None, Some(2.0)),
        None,
    );

    // Inclusive edges: exact min/max pass, the adjacent floats do not.
    let edged = index_of(&[declaration(KEY, Some(-2.5), Some(2.5))]);
    assert!(edged.check(KEY, &ConfigValue::Float(-2.5)).is_ok());
    assert!(edged.check(KEY, &ConfigValue::Float(2.5)).is_ok());
    assert!(edged.check(KEY, &ConfigValue::Int(-2)).is_ok());
    assert!(edged.check(KEY, &ConfigValue::Int(2)).is_ok());
    let below_min = (-2.5_f64).next_down();
    let above_max = 2.5_f64.next_up();
    assert_out_of_range(
        &edged,
        KEY,
        &ConfigValue::Float(below_min),
        below_min,
        (Some(-2.5), Some(2.5)),
        None,
    );
    assert_out_of_range(
        &edged,
        KEY,
        &ConfigValue::Float(above_max),
        above_max,
        (Some(-2.5), Some(2.5)),
        None,
    );

    // A contradictory intersection rejects every finite value.
    let contradictory = index_of(&[
        declaration(KEY, Some(0.0), Some(5.0)),
        declaration(KEY, Some(10.0), Some(100.0)),
    ]);
    for value in [0.0, 3.0, 7.0, 50.0, -1.0e300, 1.0e300] {
        assert_out_of_range(
            &contradictory,
            KEY,
            &ConfigValue::Float(value),
            value,
            (Some(10.0), Some(5.0)),
            None,
        );
    }

    // Empty lists pass; the first offending element is reported by position.
    let listed = index_of(&[declaration(KEY, Some(0.0), Some(1.0))]);
    assert!(
        listed.check(KEY, &ConfigValue::List(vec![])).is_ok(),
        "an empty list has no offending element"
    );
    let first_at_zero = ConfigValue::List(vec![ConfigValue::Float(2.0)]);
    assert_out_of_range(
        &listed,
        KEY,
        &first_at_zero,
        2.0,
        (Some(0.0), Some(1.0)),
        Some(0),
    );
    let first_at_two = ConfigValue::List(vec![
        ConfigValue::Float(0.5),
        ConfigValue::Int(1),
        ConfigValue::Float(-1.0),
    ]);
    assert_out_of_range(
        &listed,
        KEY,
        &first_at_two,
        -1.0,
        (Some(0.0), Some(1.0)),
        Some(2),
    );

    // Bool and String pass this bounds-only surface.
    assert!(listed.check(KEY, &ConfigValue::Bool(true)).is_ok());
    assert!(listed
        .check(KEY, &ConfigValue::String("grid".to_string()))
        .is_ok());

    // Percent and percent-authored FloatOrPercent enforce the min side only.
    let percented = index_of(&[declaration(KEY, Some(0.0), Some(10.0))]);
    assert!(
        percented.check(KEY, &ConfigValue::Percent(50.0)).is_ok(),
        "a raw percentage above the absolute maximum must still pass: the maximum is Phase-B's"
    );
    assert!(
        percented
            .check(
                KEY,
                &ConfigValue::FloatOrPercent {
                    value: 300.0,
                    is_percent: true,
                },
            )
            .is_ok(),
        "a percent-authored FloatOrPercent above the absolute maximum must still pass"
    );
    assert_out_of_range(
        &percented,
        KEY,
        &ConfigValue::Percent(-5.0),
        -5.0,
        (Some(0.0), Some(10.0)),
        None,
    );
    assert_out_of_range(
        &percented,
        KEY,
        &ConfigValue::FloatOrPercent {
            value: -1.0,
            is_percent: true,
        },
        -1.0,
        (Some(0.0), Some(10.0)),
        None,
    );

    // An absolute FloatOrPercent applies the full bounds.
    assert_out_of_range(
        &percented,
        KEY,
        &ConfigValue::FloatOrPercent {
            value: 50.0,
            is_percent: false,
        },
        50.0,
        (Some(0.0), Some(10.0)),
        None,
    );
    assert!(percented
        .check(
            KEY,
            &ConfigValue::FloatOrPercent {
                value: 10.0,
                is_percent: false,
            },
        )
        .is_ok());

    // Non-finite absolute scalars reject and echo the declared bounds.
    assert_out_of_range(
        &percented,
        KEY,
        &ConfigValue::Float(f64::NEG_INFINITY),
        f64::NEG_INFINITY,
        (Some(0.0), Some(10.0)),
        None,
    );
    assert_out_of_range(
        &percented,
        KEY,
        &ConfigValue::Float(f64::NAN),
        f64::NAN,
        (Some(0.0), Some(10.0)),
        None,
    );
}

fn declaration(key: &str, min: Option<f64>, max: Option<f64>) -> BoundsDeclaration {
    BoundsDeclaration {
        key: key.to_string(),
        min,
        max,
        module_id: "mod_a".to_string(),
    }
}

fn index_of(declarations: &[BoundsDeclaration]) -> ConfigBoundsIndex {
    ConfigBoundsIndex::from_declarations(declarations.iter().cloned())
}

fn assert_out_of_range(
    index: &ConfigBoundsIndex,
    key: &str,
    value: &ConfigValue,
    expected_value: f64,
    expected_bounds: (Option<f64>, Option<f64>),
    expected_index: Option<usize>,
) {
    match index.check(key, value) {
        Ok(()) => panic!(
            "expected OutOfRange for key {key:?} value {value:?}, got Ok (declared bounds {expected_bounds:?})"
        ),
        Err(ConfigResolutionError::OutOfRange {
            key: actual_key,
            value: actual_value,
            min,
            max,
            index: actual_index,
        }) => {
            assert_eq!(actual_key, key, "OutOfRange key for {value:?}");
            assert_eq!(
                actual_value.to_bits(),
                expected_value.to_bits(),
                "OutOfRange value bits for {value:?}"
            );
            assert_eq!(
                min.map(f64::to_bits),
                expected_bounds.0.map(f64::to_bits),
                "OutOfRange min for {value:?}"
            );
            assert_eq!(
                max.map(f64::to_bits),
                expected_bounds.1.map(f64::to_bits),
                "OutOfRange max for {value:?}"
            );
            assert_eq!(
                actual_index, expected_index,
                "OutOfRange index for {value:?}"
            );
        }
        Err(other) => panic!("expected OutOfRange for {value:?}, got {other:?}"),
    }
}

/// Deterministic population guard: the strategy must reach every mode the
/// property claims to explore within a bounded pre-sample.
fn assert_substantive_population<S>(strategy: &S)
where
    S: Strategy<Value = (Vec<BoundsDeclaration>, String, ConfigValue)>,
{
    let mut runner = TestRunner::new(Config {
        cases: PRE_SAMPLE_CASES as u32,
        max_local_rejects: 0,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5eed_c0de),
        failure_persistence: None,
        ..Config::default()
    });
    let samples: Vec<(Vec<BoundsDeclaration>, String, ConfigValue)> = (0..PRE_SAMPLE_CASES)
        .map(|_| {
            strategy
                .new_tree(&mut runner)
                .expect("the declaration/value strategy must not reject")
                .current()
        })
        .collect();

    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    let mut deferred_index = 0_usize;
    let mut non_finite = 0_usize;
    let mut index_zero_witness: Option<String> = None;
    for (declarations, key, value) in &samples {
        let index = ConfigBoundsIndex::from_declarations(declarations.iter().cloned());
        match index.check(key, value) {
            Ok(()) => accepted += 1,
            Err(ConfigResolutionError::OutOfRange {
                index: Some(position),
                ..
            }) => {
                rejected += 1;
                if position >= 1 {
                    deferred_index += 1;
                } else if index_zero_witness.is_none() {
                    index_zero_witness = Some(format!(
                        "key {key:?} value {value:?} declarations {}",
                        describe(declarations)
                    ));
                }
            }
            Err(_) => rejected += 1,
        }
        if contains_non_finite(value) {
            non_finite += 1;
        }
    }

    assert!(
        accepted > 0,
        "pre-sample of {PRE_SAMPLE_CASES} never produced an accepted case (rejected {rejected})"
    );
    assert!(
        rejected > 0,
        "pre-sample of {PRE_SAMPLE_CASES} never produced a rejected case (accepted {accepted})"
    );
    assert!(
        deferred_index > 0,
        "pre-sample of {PRE_SAMPLE_CASES} never produced a list rejection at index 1 or later \
         (rejected {rejected}); the first invalid position is only exercised at index 0. \
         Index-0 witness: {}",
        index_zero_witness.as_deref().unwrap_or("none")
    );
    assert!(
        non_finite > 0,
        "pre-sample of {PRE_SAMPLE_CASES} never produced a non-finite value (accepted {accepted})"
    );
}

fn contains_non_finite(value: &ConfigValue) -> bool {
    match value {
        ConfigValue::Float(value) | ConfigValue::Percent(value) => !value.is_finite(),
        ConfigValue::FloatOrPercent { value, .. } => !value.is_finite(),
        ConfigValue::List(elements) => elements.iter().any(contains_non_finite),
        ConfigValue::Bool(_) | ConfigValue::Int(_) | ConfigValue::String(_) => false,
    }
}
