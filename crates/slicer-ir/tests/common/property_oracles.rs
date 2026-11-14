//! Independent reference models shared by the `slicer-ir` property targets.
//!
//! Included by both `tests/property_ir.rs` and
//! `tests/property_ir_contracts.rs` through
//! `#[path = "common/property_oracles.rs"] mod property_oracles;`. This is not
//! a cargo target: the deterministic controls in `property_ir_contracts.rs`
//! must run under a plain `cargo test`, so nothing here reads
//! `PNP_PROPERTY_CONTEXT`, builds a `PropertyRun`, or consults the property
//! registry.
//!
//! Every model below is derived from the charter text (or from the documented
//! representation contract), not from production output. Helpers whose name
//! starts with `defective_` are deliberate mutations: they exist so the
//! contract tests can prove the oracle rejects a plausible fault.
//!
//! Each test binary consumes a subset of these items. Consumers declare
//! `#[allow(dead_code)]` on their `#[path = "..."] mod property_oracles;`
//! declaration; this file deliberately carries no module-level attribute of its
//! own, because combining the two triggers `clippy::duplicated_attributes`.

use std::collections::BTreeMap;

use slicer_ir::{
    mm_to_units, units_to_mm, ConfigId, ConfigValue, EntitySpeedProfile, ExPolygon, ExtrusionRole,
    LayerCollectionIR, PaintValue, Point2, Polygon, PrintEntity, ResolvedConfig, TravelMove,
    MODIFIER_FOOTPRINT_REGION_ID, MODIFIER_VARIANT_REGION_ID_STRIDE,
};
use slicer_sdk::coords::{mm_to_units as sdk_mm_to_units, units_to_mm as sdk_units_to_mm};
use slicer_sdk::test_support::fixtures::print_entity_base;

/// Audit a **composed** owner strategy: generate several cases, run real
/// `simplify()` calls, and assert the case's mode tag never changes.
///
/// The mechanism probe in `property_ir_contracts.rs` covers `mode_locked`
/// itself; this helper runs inside each property so the actual composed
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
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("composed strategy tree");
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

/// A strategy whose *mode* is pinned for each generated case while its payload
/// still shrinks.
///
/// `prop_oneof!` unions shrink into earlier alternatives, so a tagged enum built
/// with `prop_oneof!` can change variant during shrinking even though the tag is
/// structural. This helper generates the mode from a `no_shrink` selection and
/// then builds the payload with `prop_flat_map`: because the mode tree can never
/// simplify, the payload tree shrinks inside the originally generated mode.
///
/// The mechanism is checked by `mode_lock_preserves_tags_under_simplification` in
/// `crates/slicer-ir/tests/property_ir_contracts.rs`.
pub fn mode_locked<M, T>(
    modes: Vec<M>,
    build: impl Fn(M) -> proptest::strategy::BoxedStrategy<T> + 'static,
) -> proptest::strategy::BoxedStrategy<T>
where
    M: Clone + std::fmt::Debug + 'static,
    T: std::fmt::Debug + 'static,
{
    use proptest::strategy::Strategy;
    proptest::sample::select(modes)
        .no_shrink()
        .prop_flat_map(move |mode| build(mode))
        .boxed()
}

// ============================================================================
// (a) Coordinate envelope oracle — charter H-COORD
// ============================================================================

/// Largest scaled magnitude the charter admits: `2^24` units = `2^24 / 10^4`
/// mm. Above this the f32 product loses the integer-resolution argument.
const SCALED_ABS_LIMIT: f64 = 16_777_216.0;

/// Cap imposed by the shared `rounding_boundary_mm` strategy
/// (`slicer_sdk::test_support::property_testing::domains::rounding_boundary_mm`
/// rejects `max_abs_units` outside `1..=1_000_000`).
const SHARED_BOUNDARY_UNIT_LIMIT: i64 = 1_000_000;

/// `2^-24`: half an f32 ulp relative error for a single correctly-rounded
/// operation.
fn f32_mul_roundoff() -> f64 {
    2f64.powi(-24)
}

/// `2^-150`: half the smallest f32 subnormal, the absolute floor for a
/// subnormal-denormalised result.
fn f32_subnormal_floor() -> f64 {
    2f64.powi(-150)
}

/// A checked millimetre/unit domain for the f32 boundary conversions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordProfile {
    /// Largest absolute millimetre input the profile admits.
    pub max_abs_mm: f64,
    /// Largest absolute integer unit the profile admits.
    pub max_abs_units: i64,
}

impl CoordProfile {
    /// Build a profile only when its safety inequalities hold.
    ///
    /// Requires a finite positive `max_abs_mm`, `max_abs_units > 0`,
    /// `10_000 * max_abs_mm <= 2^24`, `max_abs_units <= 2^24`, and
    /// `min(max_abs_units, 10_000 * max_abs_mm) <= 1_000_000` so the shared
    /// rounding-boundary strategy can cover the whole domain.
    ///
    /// Unsafe profiles are rejected, never truncated.
    pub fn new(max_abs_mm: f64, max_abs_units: i64) -> Result<Self, String> {
        if !max_abs_mm.is_finite() || max_abs_mm <= 0.0 {
            return Err(format!(
                "max_abs_mm must be finite and positive, got {max_abs_mm:?}"
            ));
        }
        if max_abs_units <= 0 {
            return Err(format!(
                "max_abs_units must be positive, got {max_abs_units}"
            ));
        }
        let scaled = 10_000.0 * max_abs_mm;
        if scaled > SCALED_ABS_LIMIT {
            return Err(format!(
                "10_000 * max_abs_mm = {scaled} exceeds 2^24 = {SCALED_ABS_LIMIT}"
            ));
        }
        if max_abs_units > SCALED_ABS_LIMIT as i64 {
            return Err(format!(
                "max_abs_units = {max_abs_units} exceeds 2^24 = {}",
                SCALED_ABS_LIMIT as i64
            ));
        }
        let boundary_units = (scaled as i64).min(max_abs_units);
        if boundary_units > SHARED_BOUNDARY_UNIT_LIMIT {
            return Err(format!(
                "min(max_abs_units, 10_000 * max_abs_mm) = {boundary_units} exceeds the \
                 shared rounding-boundary cap {SHARED_BOUNDARY_UNIT_LIMIT}"
            ));
        }
        Ok(Self {
            max_abs_mm,
            max_abs_units,
        })
    }
}

/// Derived absolute error budget, in units, for `mm_to_units(m)`:
/// `1/2 + 2^-24 * |10_000 * m| + 2^-150`, instantiated at the profile's
/// `max_abs_mm`.
///
/// Derivation: `t = 10_000 * m` is exact in f64 on this domain (a 24-bit
/// significand times the 10-bit odd factor 625 fits 53 bits), the f32 product
/// `m * 10_000` commits at most `2^-24 * |t| + 2^-150`, and rounding to the
/// nearest integer adds at most `1/2`.
pub fn mm_to_units_allowed_error(profile: &CoordProfile) -> f64 {
    // Outward-rounded: the comparison must never be tighter than the derivation,
    // so the last float below the derived value is not used.
    (0.5 + f32_mul_roundoff() * (10_000.0 * profile.max_abs_mm) + f32_subnormal_floor()).next_up()
}

/// Integer-safe form of [`mm_to_units_allowed_error`]: the smallest integer
/// that is at least the derived budget.
pub fn mm_bound(profile: &CoordProfile) -> i128 {
    mm_to_units_allowed_error(profile).ceil() as i128
}

/// Derived absolute error budget, in millimetres, for `units_to_mm(n)`:
/// `(2^-24 + 2^-53) * (max_abs_units / 10_000) + 2^-150`.
///
/// Derivation, with the exact real value `x = n / 10_000`:
///
/// - `n as f32` is exact because `|n| <= 2^24`;
/// - dividing by the exactly-represented `10_000f32` commits at most one half
///   ulp of the correctly-rounded f32 result, i.e. `2^-24 * |x|` relative;
/// - the *reference* the oracle compares against is
///   `n as f64 / 10_000.0`, which is itself a correctly-rounded f64 division
///   and therefore carries its own `2^-53 * |x|` relative error. That term is
///   not below `2^-150` (for `n = 1` the reference error is about `4.8e-21`,
///   while `2^-150` is about `7.0e-46`), so it must be added explicitly rather
///   than absorbed by the subnormal floor;
/// - `2^-150` remains only as the floor for subnormal f32 results, where a
///   relative bound underflows to zero.
///
/// The budget is therefore `(2^-24 + 2^-53) * |x| + 2^-150`, not
/// `2^-24 * |x| + 2^-150`.
pub fn units_bound(profile: &CoordProfile) -> f64 {
    let magnitude = profile.max_abs_units as f64 / 10_000.0;
    // Outward-rounded for the same reason as `mm_to_units_allowed_error`.
    ((f32_mul_roundoff() + f64_div_roundoff()) * magnitude + f32_subnormal_floor()).next_up()
}

/// f64 division relative rounding bound: one half ulp, `2^-53`.
fn f64_div_roundoff() -> f64 {
    2f64.powi(-53)
}

/// Round a real value to the nearest integer, breaking exact ties away from
/// zero (the canonical `f32::round` rule).
///
/// Implemented with magnitude-plus-one-half flooring so it is an independent
/// formulation of the production `(x).round()` call rather than a restatement
/// of it. Nonfinite values saturate exactly like an `as i64` cast.
pub fn round_half_away_from_zero(value: f64) -> i64 {
    let magnitude = value.abs();
    if !magnitude.is_finite() {
        return if value.is_nan() {
            0
        } else if value.is_sign_negative() {
            i64::MIN
        } else {
            i64::MAX
        };
    }
    let rounded = (magnitude + 0.5).floor();
    let signed = if value.is_sign_negative() {
        -rounded
    } else {
        rounded
    };
    signed as i64
}

/// The value the derivation says `mm_to_units(m)` must produce: round the
/// exact f64 scaled value half away from zero.
pub fn expected_half_away(m: f32) -> i64 {
    round_half_away_from_zero(10_000.0 * f64::from(m))
}

/// Check an arbitrary `m -> units` implementation against the derived
/// envelope and the exact half-away tie rule.
///
/// This is the callable checker `check_mm_case` delegates to; the contract test
/// points it at defective implementations to prove it can fail.
pub fn check_mm_case_with<F: Fn(f32) -> i64>(
    profile: &CoordProfile,
    m: f32,
    convert: F,
) -> Result<(), String> {
    let actual = convert(m);
    check_mm_units_envelope(profile, m, actual)?;
    check_exact_half_away(m, actual)
}

/// Check `mm_to_units(m)` against the derived envelope, the exact half-away
/// tie rule, and the (consistency-only) SDK/`Point2` round-trip agreement.
///
/// Wrapper and `Point2` agreement is consistency evidence, not correctness
/// evidence: all of them delegate to the same arithmetic, so they agree even
/// when the arithmetic is wrong. The envelope and the half-away tie are the
/// correctness claim. `Point2::to_mm` is exercised here so the declared
/// `Point2::{from_mm, to_mm}` owner surface is covered, and its float bits are
/// compared against `units_to_mm` exactly.
pub fn check_mm_case(profile: &CoordProfile, m: f32) -> Result<(), String> {
    check_mm_case_with(profile, m, mm_to_units)?;
    let actual = mm_to_units(m);
    if sdk_mm_to_units(m) != actual {
        return Err(format!(
            "slicer_sdk::coords::mm_to_units({m:?}) = {} disagrees with slicer_ir::mm_to_units = {actual}",
            sdk_mm_to_units(m)
        ));
    }
    let point = Point2::from_mm(m, -m);
    if point.x != actual || point.y != -actual {
        return Err(format!(
            "Point2::from_mm({m:?}, {:?}) = ({}, {}) disagrees with slicer_ir::mm_to_units = {actual}",
            -m, point.x, point.y
        ));
    }
    let (x_mm, y_mm) = point.to_mm();
    if x_mm.to_bits() != units_to_mm(point.x).to_bits()
        || y_mm.to_bits() != units_to_mm(point.y).to_bits()
    {
        return Err(format!(
            "Point2::to_mm at {m:?} produced ({x_mm:?}, {y_mm:?}), which disagrees bitwise with \
             units_to_mm({}, {})",
            point.x, point.y
        ));
    }
    if sdk_units_to_mm(point.x) != units_to_mm(point.x)
        || sdk_units_to_mm(point.y) != units_to_mm(point.y)
    {
        return Err(format!(
            "slicer_sdk::coords::units_to_mm disagrees with slicer_ir::units_to_mm at {m:?}"
        ));
    }
    Ok(())
}

/// The derived envelope alone: `|actual - 10_000 * m| <= 1/2 + e_mul`.
///
/// Deliberately tie-blind: at an exact half-unit input the truncating mutant is
/// exactly `1/2` away and therefore *inside* this envelope. That blindness is
/// why the charter also requires the exact half-away witnesses.
pub fn check_mm_units_envelope(
    profile: &CoordProfile,
    m: f32,
    actual_units: i64,
) -> Result<(), String> {
    let value = f64::from(m);
    if !value.is_finite() || value.abs() > profile.max_abs_mm {
        return Err(format!(
            "{m:?} is outside the profile's +/-{} mm domain",
            profile.max_abs_mm
        ));
    }
    let exact = 10_000.0 * value;
    let allowed = mm_to_units_allowed_error(profile);
    let error = (actual_units as f64 - exact).abs();
    if error > allowed {
        return Err(format!(
            "converted units {actual_units} are {error} away from the exact scaled value {exact}; \
             the derived envelope allows {allowed} (integer bound {})",
            mm_bound(profile)
        ));
    }
    Ok(())
}

/// Exact half-away tie rule, applied only when the exact scaled value is a
/// genuine half-unit tie.
fn check_exact_half_away(m: f32, actual_units: i64) -> Result<(), String> {
    let exact = 10_000.0 * f64::from(m);
    if exact.fract().abs() != 0.5 {
        return Ok(());
    }
    let expected = round_half_away_from_zero(exact);
    if actual_units != expected {
        return Err(format!(
            "converted units {actual_units} disagree with the exact half-unit tie: \
             {exact} must round away from zero to {expected}"
        ));
    }
    Ok(())
}

/// Check `units_to_mm(n)` against its derived millimetre envelope and domain.
pub fn check_units_case(profile: &CoordProfile, n: i64) -> Result<(), String> {
    if n.unsigned_abs() as u128 > profile.max_abs_units as u128 {
        return Err(format!(
            "{n} is outside the profile's +/-{} unit domain",
            profile.max_abs_units
        ));
    }
    let actual = f64::from(units_to_mm(n));
    let exact = n as f64 / 10_000.0;
    let allowed = units_bound(profile);
    let error = (actual - exact).abs();
    if error > allowed {
        return Err(format!(
            "units_to_mm({n}) = {actual} is {error} mm away from {exact}; the derived envelope \
             allows {allowed}"
        ));
    }
    Ok(())
}

/// Check that `mm_to_units` is monotone on an ordered pair inside the profile.
pub fn check_monotonic_pair(profile: &CoordProfile, a: f32, b: f32) -> Result<(), String> {
    if a > b {
        return Err(format!("pair ({a:?}, {b:?}) is not ordered a <= b"));
    }
    check_mm_units_envelope(profile, a, mm_to_units(a))?;
    check_mm_units_envelope(profile, b, mm_to_units(b))?;
    let (units_a, units_b) = (mm_to_units(a), mm_to_units(b));
    if units_a > units_b {
        return Err(format!(
            "monotonicity violated: {a:?} -> {units_a} exceeds {b:?} -> {units_b}"
        ));
    }
    Ok(())
}

/// Exact half-unit witnesses, each paired with the value the *derivation*
/// produces (`expected_half_away`), never with copied production output.
///
/// `1/32 mm` is exactly `312.5` units, so half-away rounding requires `313`;
/// the negative case requires `-313`. The neighbour floats sit one f32 ulp
/// either side of the tie and must follow their own exact scaled value. Both
/// zero signs map to `0`.
pub fn half_unit_witnesses() -> Vec<(f32, i64)> {
    let center = 1.0f32 / 32.0;
    [
        center,
        -center,
        center.next_down(),
        center.next_up(),
        (-center).next_down(),
        (-center).next_up(),
        0.0,
        -0.0,
    ]
    .into_iter()
    .map(|value| (value, expected_half_away(value)))
    .collect()
}

/// Defective reference: scale by `10^6` instead of `10^4` (a factor-of-100
/// error).
pub fn defective_factor_100(m: f32) -> i64 {
    (m * 1_000_000.0).round() as i64
}

/// Defective reference: truncate toward zero instead of rounding to nearest.
pub fn defective_truncating(m: f32) -> i64 {
    (f64::from(m) * 10_000.0).trunc() as i64
}

// ============================================================================
// (b) ID / modifier-parent oracle — charter H-IDS
// ============================================================================

/// Independent derivation of the representable parent limit
/// `P = floor((2^63 - S - 1) / S)` with `S = MODIFIER_VARIANT_REGION_ID_STRIDE`.
///
/// Computed with checked u128 arithmetic at runtime; never a hard-coded
/// literal.
pub fn representable_parent_limit() -> u64 {
    let bound: u128 = 1 << 63;
    let stride = MODIFIER_VARIANT_REGION_ID_STRIDE as u128;
    ((bound - stride - 1) / stride) as u64
}

/// Independent model of `modifier_sub_region_id_fits`: the documented
/// worst-case payload `parent * S + (S - 1)` fits strictly below the reserved
/// sentinel `2^63 - 1`.
pub fn payload_fits_model(parent: u64) -> bool {
    let stride = MODIFIER_VARIANT_REGION_ID_STRIDE as u128;
    parent as u128 * stride + (stride - 1) < (1u128 << 63) - 1
}

/// Inclusive payload interval the quotient/remainder model permits for
/// `parent` and a candidate remainder `hash`:
/// `payload = parent * S + hash` with `1 <= hash <= S - 1`.
///
/// Returns `(parent * S + hash, parent * S + (S - 1))`. Callers must assert
/// `hash` itself lies in `1..S`; a remainder of `0` (or one at or above `S`)
/// makes the interval fail the caller's own bound check rather than panicking
/// here on what would be a system-under-test defect.
pub fn expected_payload_bounds(parent: u64, hash: u64) -> (u64, u64) {
    let stride = MODIFIER_VARIANT_REGION_ID_STRIDE as u128;
    let base = parent as u128 * stride;
    ((base + hash as u128) as u64, (base + stride - 1) as u64)
}

/// Decode a high-bit-marked, non-sentinel modifier id into
/// `(parent, remainder)` using division and remainder only.
///
/// `None` means "not in the modifier namespace": an ordinary low-bit id or the
/// reserved footprint sentinel. `Some(parent)` is *not* provenance validation —
/// any bit-marked value decodes, whether or not it was ever minted.
pub fn decode_model(id: u64) -> Option<(u64, u64)> {
    if id == MODIFIER_FOOTPRINT_REGION_ID || id >> 63 != 1 {
        return None;
    }
    let payload = id & !(1u64 << 63);
    Some((
        payload / MODIFIER_VARIANT_REGION_ID_STRIDE,
        payload % MODIFIER_VARIANT_REGION_ID_STRIDE,
    ))
}

/// The mathematical allocation sequence for `n` fresh allocations:
/// `1..=n`.
pub fn allocation_model(n: u64) -> Vec<u64> {
    (1..=n).collect()
}

/// Integer-grid `ExPolygon` batch for modifier id inputs.
///
/// `points` is the outer contour; each entry of `holes` becomes one hole ring.
/// An empty `points` slice yields no polygons at all. No float conversion is
/// involved, so the geometry is exactly reproducible.
pub fn footprint(points: &[(i64, i64)], holes: &[Vec<(i64, i64)>]) -> Vec<ExPolygon> {
    if points.is_empty() {
        return Vec::new();
    }
    vec![ExPolygon {
        contour: Polygon {
            points: points
                .iter()
                .map(|&(x, y)| Point2 { x, y })
                .collect::<Vec<_>>(),
        },
        holes: holes
            .iter()
            .map(|hole| Polygon {
                points: hole
                    .iter()
                    .map(|&(x, y)| Point2 { x, y })
                    .collect::<Vec<_>>(),
            })
            .collect(),
    }]
}

// ============================================================================
// (c) Config representation oracle — charter H-CONFIG-INTERN
// ============================================================================

/// Canonical Postcard representation of a resolved config.
///
/// This is the premise the equality oracle rests on: on `ResolvedConfig`'s
/// declared serializable structure, Postcard preserves scalar float bits
/// (it writes `to_bits()` byte-for-byte) and ordered `BTreeMap` content
/// (`extensions`). It has no `HashMap` field. The contract test validates the
/// premise with signed-zero, NaN-payload, option, list and extension
/// counterexamples before relying on it; a premise failure must be fixed here,
/// not worked around in the property.
pub fn representation_key(rc: &ResolvedConfig) -> Vec<u8> {
    postcard::to_allocvec(rc).expect("ResolvedConfig is a Postcard-serializable structure")
}

/// The system-under-test equality this oracle is checked against.
pub fn struct_eq(a: &ResolvedConfig, b: &ResolvedConfig) -> bool {
    a == b
}

/// Deterministic hash observation for the implication
/// `actual_eq(a, b) => hash_of(a) == hash_of(b)`.
///
/// Uses `DefaultHasher`, whose state is fixed for a given stdlib build: this is
/// a within-run observation, never a persisted digest, and unequal values are
/// never required to hash differently.
pub fn hash_of(rc: &ResolvedConfig) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    rc.hash(&mut hasher);
    hasher.finish()
}

/// First-occurrence interner model over [`representation_key`] equality.
///
/// Index 0 is always `default` (the live `RegionMapIR::default()` seeds its
/// pool with `ResolvedConfig::default()`), and each input takes the rank of
/// the first pool entry whose representation key matches. Returns the per-input
/// `ConfigId`s plus the expected pool contents.
pub fn pool_model(
    default: &ResolvedConfig,
    items: &[ResolvedConfig],
) -> (Vec<ConfigId>, Vec<ResolvedConfig>) {
    let mut pool = vec![default.clone()];
    let mut keys = vec![representation_key(default)];
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let key = representation_key(item);
        let index = match keys.iter().position(|existing| existing == &key) {
            Some(index) => index,
            None => {
                pool.push(item.clone());
                keys.push(key);
                pool.len() - 1
            }
        };
        ids.push(ConfigId(
            u32::try_from(index).expect("pool index must fit the u32 ConfigId payload"),
        ));
    }
    (ids, pool)
}

/// Set the `f64`-backed `layer_height` field.
pub fn with_f64_field(base: &ResolvedConfig, value: f64) -> ResolvedConfig {
    let mut out = base.clone();
    out.layer_height = value;
    out
}

/// Set the `f32`-backed `outer_wall_speed` field.
pub fn with_f32_field(base: &ResolvedConfig, value: f32) -> ResolvedConfig {
    let mut out = base.clone();
    out.outer_wall_speed = value;
    out
}

/// Set the `ResolvedFloatOrPercent`-backed `initial_layer_line_width` field.
pub fn with_float_or_percent(
    base: &ResolvedConfig,
    value: f64,
    is_percent: bool,
) -> ResolvedConfig {
    let mut out = base.clone();
    out.initial_layer_line_width =
        slicer_ir::resolved_config::ResolvedFloatOrPercent { value, is_percent };
    out
}

/// Set the optional `f32`-backed `arachne_min_feature_size` field.
pub fn with_option(base: &ResolvedConfig, value: Option<f32>) -> ResolvedConfig {
    let mut out = base.clone();
    out.arachne_min_feature_size = value;
    out
}

/// Rebuild the `Vec<f64>`-backed `bed_shape` sequence with `len` deterministic
/// entries. Bounds the allocation to `len` elements.
pub fn with_sequence(base: &ResolvedConfig, len: usize) -> ResolvedConfig {
    let mut out = base.clone();
    out.bed_shape = (0..len).map(|index| index as f64 * 0.5).collect();
    out
}

/// Rebuild the `Vec<String>`-backed `fill_authored_coloring` sequence with
/// `len` deterministic entries.
pub fn with_text_sequence(base: &ResolvedConfig, len: usize) -> ResolvedConfig {
    let mut out = base.clone();
    out.fill_authored_coloring = (0..len)
        .map(|index| format!("claim:fill-{index}"))
        .collect();
    out
}

/// Total `ConfigValue` list elements produced by a `with_extension` tree of
/// the requested depth and breadth.
///
/// A tree of depth `d` and breadth `b` allocates one list per non-leaf level,
/// so the count is `b + b^2 + ... + b^d` — **not** `b^d`, which counts only the
/// innermost leaves. The packet's `max_list_len` bounds *total* generated list
/// elements, so callers must budget with this sum.
///
/// Returns `None` when the sum overflows `usize`.
pub fn extension_node_count(depth: usize, breadth: usize) -> Option<usize> {
    let mut total: usize = 0;
    let mut level: usize = 1;
    for _ in 0..depth {
        level = level.checked_mul(breadth)?;
        total = total.checked_add(level)?;
    }
    Some(total)
}

/// Clamp an extension shape so its **total** list-element count fits `budget`.
///
/// Depth is first clamped to the budget (`extension_node_count(d, 1) == d`, so a
/// depth of `budget` is always satisfiable), then breadth is reduced until
/// `extension_node_count(depth, breadth) <= budget`, never below one. This is the
/// only place a caller should derive an extension shape from a total-element
/// budget, so the `breadth^depth` leaf miscount cannot reappear.
pub fn clamped_extension_shape(depth: usize, breadth: usize, budget: usize) -> (usize, usize) {
    // `extension_node_count(d, 1) == d`, so clamping the depth to the budget is
    // always sufficient: an otherwise impossible request such as depth 5 against
    // a budget of 4 must reduce the depth, not silently allocate 5 elements.
    let depth = depth.min(budget);
    let mut breadth = breadth.max(1);
    while extension_node_count(depth, breadth).is_none_or(|nodes| nodes > budget) {
        if breadth == 1 {
            break;
        }
        breadth -= 1;
    }
    (depth, breadth)
}

/// Count the generated `ConfigValue::List` elements carried by a configuration:
/// every level of the extension tree plus the two sequence lengths.
///
/// The packet's `max_list_len` bounds **total** elements, so a per-node
/// branching bound is not sufficient; this counter is what the regression
/// asserts against.
pub fn config_list_elements(config: &ResolvedConfig) -> usize {
    fn count_value(value: &ConfigValue) -> usize {
        match value {
            ConfigValue::List(elements) => {
                elements.len() + elements.iter().map(count_value).sum::<usize>()
            }
            _ => 0,
        }
    }
    config.extensions.values().map(count_value).sum::<usize>()
        + config.bed_shape.len()
        + config.fill_authored_coloring.len()
}

/// Populate `extensions` with a nested list of the requested depth and
/// breadth.
///
/// `depth` wrapper levels are added around an innermost `ConfigValue::Int(1)`;
/// each level holds `max(breadth, 1)` copies of the previous level, so the
/// number of list elements is `b + b^2 + ... + b^depth`
/// ([`extension_node_count`]). Callers bound both to keep allocation explicit. `ResolvedConfig::extensions` is
/// `BTreeMap<String, ConfigValue>` (`crates/slicer-ir/src/resolved_config.rs`),
/// so entry order is canonical and Postcard-stable.
pub fn with_extension(base: &ResolvedConfig, depth: usize, breadth: usize) -> ResolvedConfig {
    let breadth = breadth.max(1);
    let mut value = ConfigValue::Int(1);
    for _ in 0..depth {
        value = ConfigValue::List((0..breadth).map(|_| value.clone()).collect());
    }
    let mut out = base.clone();
    out.extensions
        .insert("property_testing.nested".to_string(), value);
    out
}

// ============================================================================
// (d) Canonical chain mixed-radix oracle — charter H-CHAINS
// ============================================================================

/// Number of chains the canonical enumeration must produce, checked in widened
/// arithmetic.
///
/// Each axis in `canonical_order` contributes radix `1 + len(values)` (and `1`
/// when the axis is absent or empty). Map keys outside `canonical_order` are
/// ignored, matching the production helper. Returns `Err` as soon as the
/// running product would exceed `max_chains`, before any output allocation.
pub fn chain_product_size(
    variants: &BTreeMap<String, Vec<PaintValue>>,
    canonical_order: &[String],
    max_chains: u128,
) -> Result<u128, String> {
    let mut product: u128 = 1;
    for axis in canonical_order {
        let radix = 1 + variants.get(axis).map_or(0, Vec::len) as u128;
        product = product
            .checked_mul(radix)
            .ok_or_else(|| format!("chain product overflows u128 at axis {axis:?}"))?;
        if product > max_chains {
            return Err(format!(
                "chain product {product} exceeds the requested max_chains {max_chains} at axis {axis:?}"
            ));
        }
    }
    Ok(product)
}

/// Ordinal-decoding model of `enumerate_canonical_chains`.
///
/// Ordinals `0..product` are decoded as mixed-radix digits with the LAST axis
/// changing fastest (the last axis is the least-significant digit). Digit `0`
/// omits the axis; digit `d > 0` selects `values[d - 1]`. Axes are emitted in
/// `canonical_order` order. This is a decode loop, not a copy of the
/// production recursive omit/include traversal.
///
/// Callers must run [`chain_product_size`] first: this function allocates
/// `product` chains and panics if the product does not fit `usize`.
pub fn chain_product_model(
    variants: &BTreeMap<String, Vec<PaintValue>>,
    canonical_order: &[String],
) -> Vec<Vec<(String, PaintValue)>> {
    let size = chain_product_size(variants, canonical_order, u128::MAX)
        .expect("a u128 product cannot overflow");
    let size = usize::try_from(size).expect("caller checked the product against max_chains");
    let radices: Vec<usize> = canonical_order
        .iter()
        .map(|axis| 1 + variants.get(axis).map_or(0, Vec::len))
        .collect();
    let mut out = Vec::with_capacity(size.min(4096));
    for ordinal in 0..size {
        let mut digits = vec![0usize; canonical_order.len()];
        let mut rest = ordinal as u128;
        for index in (0..canonical_order.len()).rev() {
            let radix = radices[index] as u128;
            digits[index] = (rest % radix) as usize;
            rest /= radix;
        }
        let mut chain = Vec::new();
        for (index, axis) in canonical_order.iter().enumerate() {
            if digits[index] > 0 {
                chain.push((axis.clone(), variants[axis][digits[index] - 1].clone()));
            }
        }
        out.push(chain);
    }
    out
}

/// Defective model: drops the empty chain from the enumeration.
pub fn defective_omit_empty_chain(
    variants: &BTreeMap<String, Vec<PaintValue>>,
    canonical_order: &[String],
) -> Vec<Vec<(String, PaintValue)>> {
    let mut model = chain_product_model(variants, canonical_order);
    model.retain(|chain| !chain.is_empty());
    model
}

/// Defective model: re-sorts the caller's value list before enumerating.
pub fn defective_sorted_values(
    variants: &BTreeMap<String, Vec<PaintValue>>,
    canonical_order: &[String],
) -> Vec<Vec<(String, PaintValue)>> {
    let sorted: BTreeMap<String, Vec<PaintValue>> = variants
        .iter()
        .map(|(axis, values)| {
            let mut values = values.clone();
            values.sort();
            (axis.clone(), values)
        })
        .collect();
    chain_product_model(&sorted, canonical_order)
}

/// Defective model: enumerates the axes in reversed canonical order.
pub fn defective_swapped_axis_order(
    variants: &BTreeMap<String, Vec<PaintValue>>,
    canonical_order: &[String],
) -> Vec<Vec<(String, PaintValue)>> {
    let reversed: Vec<String> = canonical_order.iter().rev().cloned().collect();
    chain_product_model(variants, &reversed)
}

/// Defective model: emits one product too few.
pub fn defective_drop_last_product(
    variants: &BTreeMap<String, Vec<PaintValue>>,
    canonical_order: &[String],
) -> Vec<Vec<(String, PaintValue)>> {
    let mut model = chain_product_model(variants, canonical_order);
    model.pop();
    model
}

/// Defective model: repeats the last product.
pub fn defective_duplicate_product(
    variants: &BTreeMap<String, Vec<PaintValue>>,
    canonical_order: &[String],
) -> Vec<Vec<(String, PaintValue)>> {
    let mut model = chain_product_model(variants, canonical_order);
    if let Some(last) = model.last().cloned() {
        model.push(last);
    }
    model
}

// ============================================================================
// (e) Anchor side-table control — charter H-IR
// ============================================================================

/// Build a layer whose travel anchor exists only in an unrelated side table.
///
/// `ordered_entities` carries `entity_ids`; `travel_moves` references
/// `travel_id`; `speed_profiles` carries a row for `travel_id`. The validator
/// under test (`slicer_ir::validate_travel_anchors`) reads
/// `layer.ordered_entities` only, so any `travel_id` absent from `entity_ids`
/// is a dangling anchor even though the side table mentions it.
pub fn layer_with_side_table_only_anchor(entity_ids: &[u64], travel_id: u64) -> LayerCollectionIR {
    LayerCollectionIR {
        ordered_entities: entity_ids
            .iter()
            .map(|&entity_id| PrintEntity {
                entity_id,
                ..print_entity_base(ExtrusionRole::OuterWall)
            })
            .collect(),
        travel_moves: vec![TravelMove {
            entity_id: travel_id,
            ..Default::default()
        }],
        speed_profiles: vec![EntitySpeedProfile {
            entity_id: travel_id,
            factors: vec![1.0],
        }],
        ..Default::default()
    }
}
