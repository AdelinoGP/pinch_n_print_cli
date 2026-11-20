//! `prepass_owned_shrink_and_bound_controls` — the owned shrink witnesses.
//!
//! AC-N2 asks for actual `current` / `simplify` / `complicate` states from
//! **nonminimal** witnesses, not a bound check. This module drives the real
//! value trees the properties consume (`Strategy::new_tree`), walks them in both
//! directions, and re-runs the same checker on every exposed state, so a model
//! that only holds at one size cannot pass.
//!
//! A property that failed here would be a real defect: the registry's shrinking
//! would stop mid-way, and the retained counterexample would be a shape the
//! model never agreed with.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, RngSeed, TestRunner};

use super::contact::{self, ContactCase};
use super::driver::{self, DriverCase};
use super::model::{self, Case};
use super::overhang::{self, OverhangCase};
use super::paint::{self, ComposeCase};
use super::roles::{self, RoleCase};
use super::support::{self, ScheduleCase};

/// How many `simplify` steps each tree may take.
const SHRINK_STEPS: usize = 12;
/// How many `complicate` steps each tree may take.
const GROW_STEPS: usize = 4;
/// Bound used for every model strategy in this control.
const BOUNDS: (usize, usize, usize, usize, usize) = (2, 2, 2, 3, 3);

fn runner(seed: u64) -> TestRunner {
    TestRunner::new(Config {
        rng_seed: RngSeed::Fixed(seed),
        cases: 1,
        ..Config::default()
    })
}

/// Drive one concrete tree in both directions.
fn walk<T, S>(
    name: &str,
    strategy: S,
    seed: u64,
    check: impl Fn(&T) -> Result<(), String>,
) -> Result<usize, String>
where
    S: Strategy<Value = T>,
    T: std::fmt::Debug,
{
    let mut runner = runner(seed);
    let mut tree = strategy
        .new_tree(&mut runner)
        .map_err(|error| format!("{name}: new_tree failed: {error}"))?;
    let current: T = tree.current();
    check(&current).map_err(|error| format!("{name}: current state: {error}"))?;

    // Grow: `complicate` must keep the model valid.
    for step in 0..GROW_STEPS {
        if !tree.complicate() {
            break;
        }
        let current: T = tree.current();
        check(&current).map_err(|error| format!("{name}: complicate step {step}: {error}"))?;
    }

    // Shrink: every accepted simplification must keep the model valid, and at
    // least one must be observed so the walk is not vacuous.
    let mut simplifications = 0usize;
    for step in 0..SHRINK_STEPS {
        if !tree.simplify() {
            break;
        }
        let current: T = tree.current();
        check(&current).map_err(|error| format!("{name}: simplify step {step}: {error}"))?;
        simplifications += 1;
    }
    Ok(simplifications)
}

/// AC-N2: every owned tree survives growth and simplification.
pub fn owned_shrink_controls() {
    let (layers, objects, regions, semantics, values) = BOUNDS;
    let mut total_simplifications = 0usize;

    for seed in [0x5eed_0001_u64, 0x5eed_0002, 0x5eed_0003] {
        let strategy = model::strategy(layers, objects, regions, semantics, values);
        total_simplifications += walk::<Case, _>("H-REGIONMAP", strategy, seed, |case| {
            let observed = model::observe(case)?;
            model::verify(case, &observed)
        })
        .expect("H-REGIONMAP shrink walk");

        let strategy = paint::strategy(layers, semantics, 16, 64);
        total_simplifications +=
            walk::<ComposeCase, _>("H-PAINT-COMPOSE", strategy, seed, |case| {
                paint::observe_and_verify(case)
            })
            .expect("H-PAINT-COMPOSE shrink walk");

        let strategy = driver::strategy(objects);
        total_simplifications += walk::<DriverCase, _>("H-PAINT-DRIVER", strategy, seed, |case| {
            driver::verify(case)
        })
        .expect("H-PAINT-DRIVER shrink walk");

        let strategy = overhang::strategy(12_000);
        total_simplifications +=
            walk::<OverhangCase, _>("H-PAINT-OVERHANG", strategy, seed, |case| {
                overhang::verify(case)
            })
            .expect("H-PAINT-OVERHANG shrink walk");

        let strategy = support::strategy(layers, objects);
        total_simplifications +=
            walk::<ScheduleCase, _>("H-SUPPORT-SCHEDULE", strategy, seed, |case| {
                support::verify(case)
            })
            .expect("H-SUPPORT-SCHEDULE shrink walk");

        let strategy = contact::strategy();
        total_simplifications +=
            walk::<ContactCase, _>("H-SUPPORT-CONTACT", strategy, seed, |case| {
                contact::verify(case)
            })
            .expect("H-SUPPORT-CONTACT shrink walk");

        let strategy = roles::strategy(8);
        total_simplifications += walk::<RoleCase, _>("H-SUPPORT-ROLES", strategy, seed, |case| {
            roles::verify(case)
        })
        .expect("H-SUPPORT-ROLES shrink walk");
    }

    assert!(
        total_simplifications > 0,
        "the shrink walks must expose at least one simplification; \
         a tree that never simplifies cannot shrink a counterexample"
    );
}

/// The narrowing-bound rejections that accompany the shrink walk.
pub fn bound_rejections() {
    // `(1 + 3)^3 = 64` is the exact widest chain product.
    model::validate_bounds(3, 3, 64).expect("64 chains is exactly the bound");
    assert!(
        model::validate_bounds(3, 3, 63).is_err(),
        "a bound below the widest reachable product must be rejected"
    );
    assert!(
        model::validate_bounds(0, 4, 0).is_err(),
        "zero chains can never bound the empty chain"
    );
}
