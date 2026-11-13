//! Independent all-declarations model for the numeric config-bounds contract.
//!
//! Shared by `tests/property_config.rs` (the registry-backed property) and
//! `tests/property_config_contracts.rs` (deterministic controls) through
//! `#[path = "common/property_config_oracles.rs"] mod property_config_oracles;`.
//! It is a plain `tests/common/` module, not its own cargo target.
//!
//! Nothing in the *reference* surface calls
//! `slicer_scheduler::config_resolution::ConfigBoundsIndex::check`, the private
//! `check_value` / `check_scalar` / `check_percent_scalar`, or the private
//! `NumericBounds::intersect`. Every expectation is recomputed by walking all
//! declarations that share the key, so a defect in the aggregation or in the
//! scalar comparison has to show up as a disagreement rather than as a
//! self-fulfilling agreement.
//!
//! The only arithmetic fact shared with production is the documented
//! `ConfigValue::Int(i64)` coercion `*i as f64` that `check_scalar` performs
//! before comparing; every generated integer magnitude is small enough for that
//! coercion to be exact, so the reference does not depend on rounding.

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use slicer_ir::ConfigValue;
use slicer_scheduler::BoundsDeclaration;
use slicer_sdk::test_support::property_testing::domains::{finite_f64, nonfinite_f64, DomainError};

/// Largest absolute bound any generator in this module accepts.
///
/// The packet's numeric charter restricts generated integer magnitudes and
/// bounds to the exactly representable `f64` integer domain, checked against
/// `2^53 - 1`. This module deliberately stops at `2^52`:
///
/// * on `[2^52, 2^53)` the `f64` spacing is exactly `1`, so `2^52` leaves one
///   full unit of headroom for the `next_up()` / `next_down()` neighbours and
///   the `+ 1` offsets this module emits, keeping every probe an exact integer;
/// * `2^52` itself is exact, and `2^52 < 2^53 - 1`, so the packet's inequality
///   holds with margin.
pub const MAX_ABS_BOUND: f64 = 4_503_599_627_370_496.0;

/// Largest declaration count `declaration_sets` accepts.
pub const MAX_DECLARATIONS: usize = 64;

/// Largest flat-list length `bound_values` accepts.
pub const MAX_LIST_LEN: usize = 64;

/// Fixed key alphabet; a case draws a contiguous window of it.
pub const KEY_ALPHABET: [&str; 8] = ["k0", "k1", "k2", "k3", "k4", "k5", "k6", "k7"];

/// Fixed module alphabet; the module id is only diagnostic.
pub const MODULE_ALPHABET: [&str; 3] = ["mod_a", "mod_b", "mod_c"];

/// Key used when a case should exercise the "no bounds declared" path.
pub const ABSENT_KEY: &str = "absent_key";

/// Deduplicated, sorted keys that occur in at least one declaration.
///
/// The result does not depend on declaration order.
#[must_use]
pub fn present_keys(declarations: &[BoundsDeclaration]) -> Vec<String> {
    let mut keys: Vec<String> = declarations
        .iter()
        .map(|declaration| declaration.key.clone())
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

/// Effective inclusive bounds for `key`, or `None` when no declaration names it.
///
/// This is the intersection computed from the declarations themselves:
/// `min` is the largest present minimum, `max` is the smallest present maximum.
/// A present declaration that supplies neither contributes nothing restrictive,
/// and duplicate keys are merged. An empty intersection (`min > max`) is
/// returned as-is; it is a real observation, not an error.
#[must_use]
pub fn expected_effective(
    declarations: &[BoundsDeclaration],
    key: &str,
) -> Option<(Option<f64>, Option<f64>)> {
    let mut present = false;
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    for declaration in declarations.iter().filter(|entry| entry.key == key) {
        present = true;
        if let Some(low) = declaration.min {
            min = Some(min.map_or(low, |current| current.max(low)));
        }
        if let Some(high) = declaration.max {
            max = Some(max.map_or(high, |current| current.min(high)));
        }
    }
    present.then_some((min, max))
}

/// Independent "the bounds API accepts this value" predicate.
///
/// Semantics derived from the declarations, not from the production matcher:
///
/// * a key named by no declaration always passes;
/// * `Bool` and `String` always pass this bounds-only surface (variant mismatch
///   belongs to `ResolvedConfig::apply_cli_key`, not to bounds checking);
/// * `Int`, `Float` and `FloatOrPercent { is_percent: false }` must be finite and
///   satisfy **every** present minimum and **every** present maximum of **every**
///   declaration for the key;
/// * `Percent` and `FloatOrPercent { is_percent: true }` must be finite and
///   satisfy every present minimum, but no maximum: the declared maximum is an
///   absolute unit and a raw percentage only acquires an absolute value during
///   Phase-B expansion;
/// * `List` requires every element to pass under the same rules.
#[must_use]
pub fn expected_accepts(
    declarations: &[BoundsDeclaration],
    key: &str,
    value: &ConfigValue,
) -> bool {
    let matching = matching_declarations(declarations, key);
    if matching.is_empty() {
        return true;
    }
    accepts_under(&matching, value)
}

/// First offending list position, or `None` when no list position offends.
///
/// Scalars (and accepted values) report `None`. A nested list reports the
/// innermost position, matching the recursive element walk production performs
/// before it raises `OutOfRange { index }`. Generated lists are flat; the nested
/// case only keeps this helper total.
#[must_use]
pub fn expected_first_invalid_index(
    declarations: &[BoundsDeclaration],
    key: &str,
    value: &ConfigValue,
) -> Option<usize> {
    let matching = matching_declarations(declarations, key);
    if matching.is_empty() {
        return None;
    }
    first_invalid_index(&matching, value)
}

/// Stable, escaped, single-line rendering of a declaration set for failure output.
///
/// Declarations are sorted, so two orderings of the same set render identically.
/// Control characters and quotes are escaped, so the rendering can never break a
/// one-line assertion message.
#[must_use]
pub fn describe(declarations: &[BoundsDeclaration]) -> String {
    let mut rendered: Vec<String> = declarations.iter().map(describe_declaration).collect();
    rendered.sort();
    format!("[{}]", rendered.join(", "))
}

/// A finite probe strictly inside the effective intersection, when one exists.
///
/// This selects *inputs*; it never certifies an outcome. `None` means the
/// intersection is empty or the key is absent, so no finite value can be shown
/// to be in range.
#[must_use]
pub fn in_range_probe(declarations: &[BoundsDeclaration], key: &str) -> Option<f64> {
    let (min, max) = expected_effective(declarations, key)?;
    match (min, max) {
        (Some(low), Some(high)) => {
            if low > high {
                None
            } else if low <= 0.0 && high >= 0.0 {
                Some(0.0)
            } else if high < 0.0 {
                Some(high)
            } else {
                Some(low)
            }
        }
        (Some(low), None) => Some(low),
        (None, Some(high)) => Some(high),
        (None, None) => Some(0.0),
    }
}

/// A finite probe outside the effective bounds, when the key declares one.
///
/// Returns `None` only for an absent key or for a declaration set that names the
/// key without any bound: such a set rejects non-finite values but no finite
/// value.
#[must_use]
pub fn out_of_range_probe(declarations: &[BoundsDeclaration], key: &str) -> Option<f64> {
    let (min, max) = expected_effective(declarations, key)?;
    match max {
        Some(high) => Some(high.next_up()),
        None => min.map(|low| low.next_down()),
    }
}

/// Scalar magnitude a `ConfigValue` reports in `OutOfRange { value }`.
///
/// Returns `None` for the variants that never reach the numeric comparison
/// (`Bool`, `String`) and for a `List`, whose reported value is one of its
/// elements.
#[must_use]
pub fn scalar_bits(value: &ConfigValue) -> Option<f64> {
    match value {
        ConfigValue::Int(value) => Some(*value as f64),
        ConfigValue::Float(value) | ConfigValue::Percent(value) => Some(*value),
        ConfigValue::FloatOrPercent { value, .. } => Some(*value),
        ConfigValue::Bool(_) | ConfigValue::String(_) | ConfigValue::List(_) => None,
    }
}

/// Whether a value stays inside the documented `bound_values` domain.
///
/// Numeric magnitudes are finite and at most `max_abs_bound` (with `Int` on the
/// exactly representable integer grid), lists are flat and at most
/// `max_list_len` long, and the non-numeric variants are unrestricted.
#[must_use]
pub fn respects_bound(value: &ConfigValue, max_abs_bound: f64, max_list_len: usize) -> bool {
    match value {
        ConfigValue::Bool(_) | ConfigValue::String(_) => true,
        ConfigValue::Int(value) => (*value as f64).abs() <= max_abs_bound,
        ConfigValue::Float(value) | ConfigValue::Percent(value) => {
            value.is_finite() && value.abs() <= max_abs_bound
        }
        ConfigValue::FloatOrPercent { value, .. } => {
            value.is_finite() && value.abs() <= max_abs_bound
        }
        ConfigValue::List(elements) => {
            elements.len() <= max_list_len
                && elements.iter().all(|element| {
                    !matches!(element, ConfigValue::List(_))
                        && respects_bound(element, max_abs_bound, max_list_len)
                })
        }
    }
}

// ── Defective mutants (contract test only) ────────────────────────────────
//
// Each is a real callable drop-in for one reference surface. They exist so the
// contract test can prove the harness actually falsifies a plausible mistake
// before the property certifies random cases. None of them is registered as a
// passing correctness property.

/// Mutant: accepts every value for every key.
#[must_use]
pub fn accept_all(_declarations: &[BoundsDeclaration], _key: &str, _value: &ConfigValue) -> bool {
    true
}

/// Mutant: aggregates bounds by union (smallest min, largest max) instead of
/// intersection, keeping the reference variant policy.
#[must_use]
pub fn union_instead_of_intersection(
    declarations: &[BoundsDeclaration],
    key: &str,
    value: &ConfigValue,
) -> bool {
    let matching = matching_declarations(declarations, key);
    if matching.is_empty() {
        return true;
    }
    let min = matching
        .iter()
        .filter_map(|declaration| declaration.min)
        .reduce(f64::min);
    let max = matching
        .iter()
        .filter_map(|declaration| declaration.max)
        .reduce(f64::max);
    bounded_accepts(min, max, value)
}

/// Mutant: treats both inclusive bounds as exclusive (`<` / `>`).
#[must_use]
pub fn off_by_one_bound(
    declarations: &[BoundsDeclaration],
    key: &str,
    value: &ConfigValue,
) -> bool {
    let matching = matching_declarations(declarations, key);
    if matching.is_empty() {
        return true;
    }
    strict_accepts_under(&matching, value)
}

/// Mutant: always forgets the first invalid list position.
#[must_use]
pub fn lose_index(
    _declarations: &[BoundsDeclaration],
    _key: &str,
    _value: &ConfigValue,
) -> Option<usize> {
    None
}

// ── Generators ────────────────────────────────────────────────────────────

/// Bounded declaration-set strategy.
///
/// Each case draws one to `max_declarations` declarations over a contiguous
/// window of [`KEY_ALPHABET`], with the key slot biased toward the window's
/// first key so duplicate-key intersections stay common. Every declaration picks
/// one of six reachable shapes: present/unbounded, min-only, max-only,
/// intersected, strictly contradictory (adjacent floats), and two arbitrary
/// one-sided values that may or may not intersect.
///
/// The strategy performs no local rejects, and every bound it emits is finite
/// with `abs(bound) <= max_abs_bound`, so shrinking can only move to another
/// admissible declaration set.
pub fn declaration_sets(
    max_declarations: usize,
    max_abs_bound: f64,
) -> Result<BoxedStrategy<Vec<BoundsDeclaration>>, DomainError> {
    validate_declaration_parameters(max_declarations, max_abs_bound)?;
    let finite = finite_f64(-max_abs_bound..=max_abs_bound)?;
    let max_keys = max_declarations.min(KEY_ALPHABET.len());
    // One slot per admissible declaration; the drawn count truncates the vec.
    let slot = (
        0usize..(2 * max_keys),
        0u8..6,
        finite.clone(),
        finite,
        0usize..MODULE_ALPHABET.len(),
    );
    let slots = proptest::collection::vec(slot, max_declarations);
    let meta = (
        0usize..max_declarations,
        0usize..KEY_ALPHABET.len(),
        0usize..KEY_ALPHABET.len(),
    );
    Ok((meta, slots)
        .prop_map(move |((count_raw, key_count_raw, key_start_raw), slots)| {
            let declaration_count = count_raw + 1;
            let key_count = 1 + key_count_raw % max_keys;
            let span = KEY_ALPHABET.len() - key_count;
            let key_start = key_start_raw % (span + 1);
            slots
                .into_iter()
                .take(declaration_count)
                .map(|(key_slot, shape, first, second, module_slot)| {
                    let key_index = if key_slot < key_count { key_slot } else { 0 };
                    let (min, max) = shape_bounds(shape, first, second, max_abs_bound);
                    BoundsDeclaration {
                        key: KEY_ALPHABET[key_start + key_index].to_string(),
                        min,
                        max,
                        module_id: MODULE_ALPHABET[module_slot].to_string(),
                    }
                })
                .collect()
        })
        .boxed())
}


/// Audit a **composed** owner strategy: generate several cases, run real
/// `simplify()` calls, and assert the case's mode tag never changes.
///
/// The mechanism probe in `property_config_contracts.rs` covers `mode_locked`
/// itself; this helper runs inside the property so the actual composed
/// strategy (payload strategies, `prop_flat_map` and all) is audited on every
/// registry execution.
pub fn assert_mode_preserved<S>(strategy: &S, label: &str, tag: impl Fn(&S::Value) -> u8)
where
    S: proptest::strategy::Strategy,
{
    use proptest::strategy::ValueTree;
    use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestRunner};
    let mut runner = TestRunner::new(Config {
        cases: 1,
        max_local_rejects: 0,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5eed_c0de),
        failure_persistence: None,
        ..Config::default()
    });
    let mut simplifications = 0_usize;
    for _ in 0..4 {
        let mut tree = strategy.new_tree(&mut runner).expect("composed strategy tree");
        let initial = tag(&tree.current());
        let mut guard = 0_usize;
        while guard < 128 && tree.simplify() {
            guard += 1;
            simplifications += 1;
            assert_eq!(
                tag(&tree.current()),
                initial,
                "{label}: shrinking changed the case mode"
            );
        }
    }
    assert!(
        simplifications > 0,
        "{label}: the composed shrink audit simplified nothing, so it certifies nothing"
    );
}

/// A strategy whose mode is pinned per generated case while its payload still
/// shrinks.
///
/// `prop_oneof!` unions shrink into earlier alternatives, so a tagged enum built
/// with `prop_oneof!` can change variant during shrinking. The mode here comes
/// from a `no_shrink` selection fed through `prop_flat_map`, so the mode tree can
/// never simplify and the payload shrinks inside the generated mode.
///
/// The mechanism is checked by `mode_lock_preserves_tags_under_simplification` in
/// `crates/slicer-scheduler/tests/property_config_contracts.rs`.
pub fn mode_locked<M, T>(
    modes: Vec<M>,
    build: impl Fn(M) -> BoxedStrategy<T> + 'static,
) -> BoxedStrategy<T>
where
    M: Clone + std::fmt::Debug + 'static,
    T: std::fmt::Debug + 'static,
{
    proptest::sample::select(modes)
        .no_shrink()
        .prop_flat_map(move |mode| build(mode))
        .boxed()
}

/// Probe families for the bounds property.
///
/// The family is pinned per case (`no_shrink` selection) and the value is
/// **derived from the current declaration set** inside `prop_map` — no
/// `prop_flat_map` re-draws it. Shrinking the declarations therefore re-derives
/// a value in the same family, and for the witness families the verdict class and
/// first-invalid index are re-derived too, so they survive shrinking by
/// construction.
#[derive(Debug, Clone, Copy)]
pub enum ProbeMode {
    /// Independent scalar on a generated or absent key.
    GeneralScalar,
    /// Independent flat list on a generated or absent key.
    GeneralList,
    /// A scalar against a key no declaration names.
    AbsentScalar,
    /// A flat list against a key no declaration names.
    AbsentList,
    /// Exactly the witness interval's inclusive minimum.
    EdgeMin,
    /// Exactly the witness interval's inclusive maximum.
    EdgeMax,
    /// The witness interval's midpoint.
    InsideScalar,
    /// A one-element list holding the midpoint.
    InsideList,
    /// One below the witness minimum: rejected.
    ViolationScalarLow,
    /// One above the witness maximum: rejected.
    ViolationScalarHigh,
    /// A list whose only invalid element is at index 0.
    ViolationListAt0,
    /// A list whose only invalid element is at index 1.
    ViolationListAt1,
    /// A list whose only invalid element is at index 2.
    ViolationListAt2,
    /// A non-finite scalar against the witness key: rejected.
    NonFiniteScalar,
    /// A non-finite one-element list against the witness key: rejected.
    NonFiniteList,
}

pub const PROBE_MODES: [ProbeMode; 15] = [
    ProbeMode::GeneralScalar,
    ProbeMode::GeneralList,
    ProbeMode::AbsentScalar,
    ProbeMode::AbsentList,
    ProbeMode::EdgeMin,
    ProbeMode::EdgeMax,
    ProbeMode::InsideScalar,
    ProbeMode::InsideList,
    ProbeMode::ViolationScalarLow,
    ProbeMode::ViolationScalarHigh,
    ProbeMode::ViolationListAt0,
    ProbeMode::ViolationListAt1,
    ProbeMode::ViolationListAt2,
    ProbeMode::NonFiniteScalar,
    ProbeMode::NonFiniteList,
];

impl ProbeMode {
    /// Scalar (0) or list (1) value shape.
    #[must_use]
    pub fn family(self) -> u8 {
        u8::from(matches!(
            self,
            ProbeMode::GeneralList
                | ProbeMode::AbsentList
                | ProbeMode::InsideList
                | ProbeMode::ViolationListAt0
                | ProbeMode::ViolationListAt1
                | ProbeMode::ViolationListAt2
                | ProbeMode::NonFiniteList
        ))
    }

    /// A key the value must be derived against: absent, generated, or witness.
    /// The key the derived value must be checked against.
    #[must_use]
    pub fn key_of(self, chosen: &str) -> String {
        match self {
            ProbeMode::AbsentScalar | ProbeMode::AbsentList => {
                ABSENT_KEY.to_string()
            }
            ProbeMode::GeneralScalar | ProbeMode::GeneralList => chosen.to_string(),
            _ => WITNESS_KEY.to_string(),
        }
    }
}

/// A list whose only invalid element sits at `index`.
fn list_with_violation(
    index: usize,
    midpoint: f64,
    violating: f64,
    max_list_len: usize,
) -> ConfigValue {
    let len = (index + 2).min(max_list_len.max(1));
    let position = index.min(len - 1);
    let mut elements = vec![ConfigValue::Float(midpoint); len];
    elements[position] = ConfigValue::Float(violating);
    ConfigValue::List(elements)
}

/// Derive the probe value from the current declarations.
///
/// Everything here is a pure function of the declaration set (plus, for the two
/// general families, an independently generated value). That is what makes the
/// shape/verdict contract hold under shrinking: the declarations shrink, this
/// function re-runs, and the same family comes out.
pub fn derive_probe(
    mode: ProbeMode,
    declarations: &[BoundsDeclaration],
    general_scalar: ConfigValue,
    general_list: ConfigValue,
    max_list_len: usize,
) -> ConfigValue {
    let (low, high) = witness_interval(declarations);
    let midpoint = (low + high) / 2.0;
    match mode {
        ProbeMode::GeneralScalar | ProbeMode::AbsentScalar => general_scalar,
        ProbeMode::GeneralList | ProbeMode::AbsentList => general_list,
        ProbeMode::EdgeMin => ConfigValue::Float(low),
        ProbeMode::EdgeMax => ConfigValue::Float(high),
        ProbeMode::InsideScalar => ConfigValue::Float(midpoint),
        ProbeMode::InsideList => ConfigValue::List(vec![ConfigValue::Float(midpoint)]),
        ProbeMode::ViolationScalarLow => ConfigValue::Float(low - 1.0),
        ProbeMode::ViolationScalarHigh => ConfigValue::Float(high + 1.0),
        ProbeMode::ViolationListAt0 => list_with_violation(0, midpoint, low - 1.0, max_list_len),
        ProbeMode::ViolationListAt1 => list_with_violation(1, midpoint, low - 1.0, max_list_len),
        ProbeMode::ViolationListAt2 => list_with_violation(2, midpoint, low - 1.0, max_list_len),
        ProbeMode::NonFiniteScalar => ConfigValue::Float(f64::NAN),
        ProbeMode::NonFiniteList => ConfigValue::List(vec![ConfigValue::Float(f64::NAN)]),
    }
}

/// Bounded config-value strategy for the bounds surface.
///
/// Produces integer-grid `Int` and `Float` values, arbitrary finite `f64`
/// values (including both zero signs, subnormal witnesses, and the
/// `next_down()` / `next_up()` neighbours of every drawn centre), `Percent` and
/// both `FloatOrPercent` modes, `Bool` / `String` witnesses, and flat `List`
/// values of length `0..=max_list_len`.
///
/// Every numeric magnitude is finite and at most `max_abs_bound`, so shrinking
/// can only move to another in-domain value.
/// The list half of [`bound_values`], for probes that must stay a flat list.
pub fn bound_list_values(
    max_abs_bound: f64,
    max_list_len: usize,
) -> Result<BoxedStrategy<ConfigValue>, DomainError> {
    validate_value_parameters(max_abs_bound, max_list_len)?;
    let elements = bound_scalar_values(max_abs_bound)?;
    Ok(proptest::collection::vec(elements, 0..=max_list_len)
        .prop_map(ConfigValue::List)
        .boxed())
}

/// A declaration key that the generator never produces, used to derive probes
/// whose expected verdict cannot be perturbed by another declaration.
pub const WITNESS_KEY: &str = "witness_bound";

/// Append a bounded, non-empty witness declaration unless one is present.
///
/// Derived probes (`Edge*`, `Inside*`, `Violation*`) need a key whose effective
/// interval is known: `WITNESS_KEY` is outside `KEY_ALPHABET`, so no generated
/// declaration can narrow it. The interval is wide enough to place both an
/// accepted midpoint and a violating neighbour, and it is re-derived from the
/// (possibly shrunk) declaration set on every case, so the probe stays
/// satisfiable under shrinking.
pub fn ensure_bounded_witness(declarations: &mut Vec<BoundsDeclaration>) {
    if declarations.iter().any(|entry| entry.key == WITNESS_KEY) {
        return;
    }
    declarations.push(BoundsDeclaration {
        key: WITNESS_KEY.to_string(),
        min: Some(-4.0),
        max: Some(4.0),
        module_id: "witness".to_string(),
    });
}

/// The inclusive interval of [`WITNESS_KEY`] after [`ensure_bounded_witness`].
pub fn witness_interval(declarations: &[BoundsDeclaration]) -> (f64, f64) {
    let (min, max) = expected_effective(declarations, WITNESS_KEY)
        .expect("the witness declaration is always present");
    (min.expect("witness min"), max.expect("witness max"))
}

pub fn bound_values(
    max_abs_bound: f64,
    max_list_len: usize,
) -> Result<BoxedStrategy<ConfigValue>, DomainError> {
    validate_value_parameters(max_abs_bound, max_list_len)?;
    let max_int = max_abs_bound as i64;
    let integer_grid = -max_int..=max_int;
    let finite = finite_f64(-max_abs_bound..=max_abs_bound)?;
    let neighbour = finite.clone();
    let clamped_down = neighbour
        .clone()
        .prop_map(move |value| ConfigValue::Float(value.next_down().max(-max_abs_bound)));
    let clamped_up =
        neighbour.prop_map(move |value| ConfigValue::Float(value.next_up().min(max_abs_bound)));
    let scalars = prop_oneof![
        3 => integer_grid.clone().prop_map(ConfigValue::Int),
        3 => integer_grid.prop_map(|value| ConfigValue::Float(value as f64)),
        4 => finite.clone().prop_map(ConfigValue::Float),
        1 => clamped_down,
        1 => clamped_up,
        1 => proptest::sample::select(subnormal_witnesses(max_abs_bound))
            .prop_map(ConfigValue::Float),
        1 => Just(ConfigValue::Float(0.0)),
        1 => Just(ConfigValue::Float(-0.0)),
        1 => finite.clone().prop_map(ConfigValue::Percent),
        1 => finite
            .clone()
            .prop_map(|value| ConfigValue::FloatOrPercent { value, is_percent: true }),
        1 => finite.prop_map(|value| ConfigValue::FloatOrPercent { value, is_percent: false }),
        1 => any::<bool>().prop_map(ConfigValue::Bool),
        1 => proptest::sample::select(vec![
            String::new(),
            "grid".to_string(),
            "lines".to_string(),
        ])
        .prop_map(ConfigValue::String),
    ];
    let lists =
        proptest::collection::vec(scalars.clone(), 0..=max_list_len).prop_map(ConfigValue::List);
    // The value family is pinned per case: a list value must not shrink into a
    // scalar (which would drop the first-invalid-index shape), and a scalar must
    // not grow into a list.
    Ok(mode_locked(vec![0_u8, 1], move |mode| match mode {
        0 => scalars.clone().boxed(),
        _ => lists.clone().boxed(),
    }))
}

/// The scalar half of [`bound_values`], for probes that must stay scalar.
pub fn bound_scalar_values(max_abs_bound: f64) -> Result<BoxedStrategy<ConfigValue>, DomainError> {
    validate_value_parameters(max_abs_bound, 1)?;
    let max_int = max_abs_bound as i64;
    let finite = finite_f64(-max_abs_bound..=max_abs_bound)?;
    let neighbour = finite.clone();
    let clamped_down = neighbour
        .clone()
        .prop_map(move |value| ConfigValue::Float(value.next_down().max(-max_abs_bound)));
    let clamped_up =
        neighbour.prop_map(move |value| ConfigValue::Float(value.next_up().min(max_abs_bound)));
    Ok(prop_oneof![
        3 => (-max_int..=max_int).prop_map(ConfigValue::Int),
        3 => (-max_int..=max_int).prop_map(|value| ConfigValue::Float(value as f64)),
        4 => finite.clone().prop_map(ConfigValue::Float),
        1 => clamped_down,
        1 => clamped_up,
        1 => proptest::sample::select(subnormal_witnesses(max_abs_bound))
            .prop_map(ConfigValue::Float),
        1 => Just(ConfigValue::Float(0.0)),
        1 => Just(ConfigValue::Float(-0.0)),
        1 => finite.clone().prop_map(ConfigValue::Percent),
        1 => finite
            .clone()
            .prop_map(|value| ConfigValue::FloatOrPercent { value, is_percent: true }),
        1 => finite.prop_map(|value| ConfigValue::FloatOrPercent { value, is_percent: false }),
        1 => any::<bool>().prop_map(ConfigValue::Bool),
        1 => proptest::sample::select(vec![
            String::new(),
            "grid".to_string(),
            "lines".to_string(),
        ])
        .prop_map(ConfigValue::String),
    ]
    .boxed())
}

/// Infinities and distinguishable NaN bit patterns in every numeric variant.
///
/// `Int` cannot carry a non-finite value, so it is absent. The `List` branch
/// holds a single non-finite `Float`, which keeps the recursive list path
/// covered without nesting.
pub fn nonfinite_values() -> BoxedStrategy<ConfigValue> {
    let exceptional = nonfinite_f64();
    let scalars = prop_oneof![
        4 => exceptional.clone().prop_map(ConfigValue::Float),
        2 => exceptional.clone().prop_map(ConfigValue::Percent),
        2 => exceptional
            .clone()
            .prop_map(|value| ConfigValue::FloatOrPercent { value, is_percent: true }),
        1 => exceptional
            .clone()
            .prop_map(|value| ConfigValue::FloatOrPercent { value, is_percent: false }),
    ]
    .boxed();
    let lists = exceptional
        .prop_map(|value| ConfigValue::List(vec![ConfigValue::Float(value)]))
        .boxed();
    // The scalar/list family is pinned so a non-finite list cannot shrink into a
    // non-finite scalar (which would drop the list path entirely).
    mode_locked(vec![0_u8, 1], move |mode| {
        if mode == 0 {
            scalars.clone()
        } else {
            lists.clone()
        }
    })
}

fn validate_declaration_parameters(
    max_declarations: usize,
    max_abs_bound: f64,
) -> Result<(), DomainError> {
    if !(1..=MAX_DECLARATIONS).contains(&max_declarations) {
        return Err(DomainError::InvalidBounds(format!(
            "declaration count must be 1..={MAX_DECLARATIONS}"
        )));
    }
    validate_bound(max_abs_bound)
}

fn validate_value_parameters(max_abs_bound: f64, max_list_len: usize) -> Result<(), DomainError> {
    if !(1..=MAX_LIST_LEN).contains(&max_list_len) {
        return Err(DomainError::InvalidBounds(format!(
            "list length must be 1..={MAX_LIST_LEN}"
        )));
    }
    validate_bound(max_abs_bound)
}

fn validate_bound(max_abs_bound: f64) -> Result<(), DomainError> {
    if !max_abs_bound.is_finite() || max_abs_bound <= 0.0 || max_abs_bound > MAX_ABS_BOUND {
        return Err(DomainError::InvalidBounds(format!(
            "absolute bound must be finite and in (0, {MAX_ABS_BOUND}]"
        )));
    }
    Ok(())
}

fn shape_bounds(
    shape: u8,
    first: f64,
    second: f64,
    max_abs_bound: f64,
) -> (Option<f64>, Option<f64>) {
    match shape {
        0 => (None, None),
        1 => (Some(first), None),
        2 => (None, Some(second)),
        3 => (Some(first.min(second)), Some(first.max(second))),
        4 => contradictory_bounds(first, max_abs_bound),
        _ => (Some(first), Some(second)),
    }
}

/// Two adjacent floats with the larger one as the minimum, so `min > max` holds
/// strictly and both stay inside `[-max_abs_bound, max_abs_bound]`.
fn contradictory_bounds(first: f64, max_abs_bound: f64) -> (Option<f64>, Option<f64>) {
    if first > -max_abs_bound {
        (Some(first), Some(first.next_down()))
    } else {
        (Some(first.next_up()), Some(first))
    }
}

fn subnormal_witnesses(max_abs_bound: f64) -> Vec<f64> {
    let mut witnesses: Vec<f64> = [
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        f64::from_bits(0x000f_ffff_ffff_ffff),
    ]
    .into_iter()
    .filter(|candidate| candidate.abs() <= max_abs_bound)
    .collect();
    if witnesses.is_empty() {
        witnesses.push(0.0);
    }
    witnesses
}

fn matching_declarations<'a>(
    declarations: &'a [BoundsDeclaration],
    key: &str,
) -> Vec<&'a BoundsDeclaration> {
    declarations
        .iter()
        .filter(|declaration| declaration.key == key)
        .collect()
}

fn accepts_under(matching: &[&BoundsDeclaration], value: &ConfigValue) -> bool {
    match value {
        ConfigValue::Bool(_) | ConfigValue::String(_) => true,
        ConfigValue::List(elements) => elements
            .iter()
            .all(|element| accepts_under(matching, element)),
        ConfigValue::Int(value) => absolute_accepts(matching, *value as f64),
        ConfigValue::Float(value) => absolute_accepts(matching, *value),
        ConfigValue::Percent(value) => percent_accepts(matching, *value),
        ConfigValue::FloatOrPercent { value, is_percent } => {
            if *is_percent {
                percent_accepts(matching, *value)
            } else {
                absolute_accepts(matching, *value)
            }
        }
    }
}

fn absolute_accepts(matching: &[&BoundsDeclaration], value: f64) -> bool {
    value.is_finite()
        && matching.iter().all(|declaration| {
            declaration.min.is_none_or(|low| value >= low)
                && declaration.max.is_none_or(|high| value <= high)
        })
}

fn percent_accepts(matching: &[&BoundsDeclaration], value: f64) -> bool {
    value.is_finite()
        && matching
            .iter()
            .all(|declaration| declaration.min.is_none_or(|low| value >= low))
}

fn first_invalid_index(matching: &[&BoundsDeclaration], value: &ConfigValue) -> Option<usize> {
    let ConfigValue::List(elements) = value else {
        return None;
    };
    for (position, element) in elements.iter().enumerate() {
        if !accepts_under(matching, element) {
            return match element {
                ConfigValue::List(_) => first_invalid_index(matching, element),
                _ => Some(position),
            };
        }
    }
    None
}

/// Variant policy for an already-aggregated `(min, max)` pair.
///
/// Only the union mutant uses it; the reference aggregates per declaration.
fn bounded_accepts(min: Option<f64>, max: Option<f64>, value: &ConfigValue) -> bool {
    match value {
        ConfigValue::Bool(_) | ConfigValue::String(_) => true,
        ConfigValue::List(elements) => elements
            .iter()
            .all(|element| bounded_accepts(min, max, element)),
        ConfigValue::Int(value) => bounded_absolute(min, max, *value as f64),
        ConfigValue::Float(value) => bounded_absolute(min, max, *value),
        ConfigValue::Percent(value) => bounded_percent(min, *value),
        ConfigValue::FloatOrPercent { value, is_percent } => {
            if *is_percent {
                bounded_percent(min, *value)
            } else {
                bounded_absolute(min, max, *value)
            }
        }
    }
}

fn bounded_absolute(min: Option<f64>, max: Option<f64>, value: f64) -> bool {
    value.is_finite() && min.is_none_or(|low| value >= low) && max.is_none_or(|high| value <= high)
}

fn bounded_percent(min: Option<f64>, value: f64) -> bool {
    value.is_finite() && min.is_none_or(|low| value >= low)
}

fn strict_accepts_under(matching: &[&BoundsDeclaration], value: &ConfigValue) -> bool {
    match value {
        ConfigValue::Bool(_) | ConfigValue::String(_) => true,
        ConfigValue::List(elements) => elements
            .iter()
            .all(|element| strict_accepts_under(matching, element)),
        ConfigValue::Int(value) => strict_absolute(matching, *value as f64),
        ConfigValue::Float(value) => strict_absolute(matching, *value),
        ConfigValue::Percent(value) => strict_percent(matching, *value),
        ConfigValue::FloatOrPercent { value, is_percent } => {
            if *is_percent {
                strict_percent(matching, *value)
            } else {
                strict_absolute(matching, *value)
            }
        }
    }
}

fn strict_absolute(matching: &[&BoundsDeclaration], value: f64) -> bool {
    value.is_finite()
        && matching.iter().all(|declaration| {
            declaration.min.is_none_or(|low| value > low)
                && declaration.max.is_none_or(|high| value < high)
        })
}

fn strict_percent(matching: &[&BoundsDeclaration], value: f64) -> bool {
    value.is_finite()
        && matching
            .iter()
            .all(|declaration| declaration.min.is_none_or(|low| value > low))
}

fn describe_declaration(declaration: &BoundsDeclaration) -> String {
    format!(
        "{{key={},min={},max={},module={}}}",
        escape(&declaration.key),
        describe_bound(declaration.min),
        describe_bound(declaration.max),
        escape(&declaration.module_id),
    )
}

fn describe_bound(bound: Option<f64>) -> String {
    match bound {
        None => "none".to_string(),
        Some(value) => format!("{value:?}"),
    }
}

fn escape(text: &str) -> String {
    text.chars().flat_map(char::escape_debug).collect()
}
