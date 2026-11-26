//! Opt-in property tests for the part-cooling layer/event decision table.
//!
//! Charter (`docs/spec_packets/proptest-openrscad_16_pbt-io-gcode/packet.spec.md`
//! § H-COOLING; registry record `H-COOLING` in `test-properties.toml`):
//!
//! * `H-COOLING` — the real private `PartCooling::layer_fan_speed` and the real
//!   public `PartCooling::cooling_decision_for_event` against an independent
//!   decision table over `fan_max_speed`, `close_fan_the_first_x_layers`,
//!   `overhang_fan_speed`, `enable_overhang_bridge_fan`, layer indices around
//!   the close boundary and bounded entity-role sequences.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `layer_fan_speed` and `is_overhang_role` are private in the crate root.
//! This module is a crate-internal unit-test child, so the private method is
//! reachable through `super::PartCooling` without widening production
//! visibility.
//!
//! # Independent expected model
//!
//! The checker never calls the module to obtain an expected value:
//!
//! ```text
//! base     = 0 if layer_index < close_layers else fan_max_speed
//! overhang = (overhang_fan_speed as u16 * fan_max_speed as u16) / 100
//! result   = overhang as u8 if enable_overhang_bridge_fan
//!                          && any(entity.role == BridgeInfill)
//!            else base
//! ```
//!
//! Exact integers, no tolerance. The multiplication happens in `u16` before
//! the `u8` cast, and the manifest ranges (`overhang_fan_speed <= 100`,
//! `fan_max_speed <= 255`) keep the product's quotient at or below 255, so no
//! wrapping or truncation is possible on the declared domain. No monotonicity
//! property is claimed: `overhang_fan_speed` is a table input, and a higher
//! percentage can leave the integer result unchanged. Only `BridgeInfill` is
//! an overhang role; `InternalBridgeInfill` is the discriminating non-overhang
//! role in the role-set control.
//!
//! The canonical reference is `CoolingBuffer::apply_layer_cooldown`'s local
//! `change_extruder_set_fan`. PnP's two-function decomposition is a simplified
//! decision table, **not** a full cooling-buffer parity claim.
//!
//! # Literal review (manual; the scanners do not cover this file)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`.
//!
//! Watched-type literal sites here (a watched type is a `pub` struct with at
//! least five named fields under `crates/*/src`):
//!
//! * `PrintEntity` (`crates/slicer-ir/src/slice_ir.rs`) — `view_for` sets
//!   `entity_id` and `path` and takes every other field from
//!   `print_entity_base(role)` (`..` FRU).
//! * `ExtrusionPath3D` — no literal: `extrusion_path3d_base(role)` is used
//!   directly.
//! * `LayerCollectionIR` — `view_for` sets `global_layer_index` and
//!   `ordered_entities` and takes the rest from `..Default::default()` (FRU).
//! * `Point3WithWidth` — not constructed here.
//!
//! `RegionKey`, `CoolingCase`, `CoolingLimits`, `CoolingModel` and the mode
//! enums are not watched crate-public structs (`RegionKey` has four named
//! fields; the rest are module-private test types), so their literals are
//! exhaustive by construction because the fields are the fixture.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap.
//! `cooling_controls` is a set of mutation controls over the **same**
//! `check_cooling_case` the property uses: each first asserts the unmodified
//! decision is accepted, then asserts the single-knob mutation is rejected by
//! the surface that disagreed (`layer_fan_speed` or
//! `cooling_decision_for_event`). The property requires non-vacuous positive
//! populations (counters asserted after the run), so a generator regression
//! that stopped producing an overhang role or a boundary layer would fail
//! rather than pass silently. The shrink probe walks a fixed-shape witness per
//! role mode and per `enable` value; it asserts the retained mode at every
//! step and never claims global minimality or a shrink-step count.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{ConfigValue, ConfigView, ExtrusionRole, LayerCollectionIR, PrintEntity};
use slicer_sdk::test_support::fixtures::{extrusion_path3d_base, print_entity_base};
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_sdk::traits::LayerCollectionView;

use super::PartCooling;

const PROPERTY_ID: &str = "H-COOLING";
const PROPERTY_TEST: &str = "property_tests::proptest_cooling_decision_table";
/// Exact `PropertyRun::bound` keys of the registry record.
const BOUND_KEYS: [&str; 2] = ["max_layer_index", "max_close_layers"];

/// Deterministic controls have no `PropertyRun` context; these are the exact
/// registry routine-profile bound values (`max_layer_index = 8`,
/// `max_close_layers = 4`; the extended profile shares them).
const CONTROL_MAX_LAYER_INDEX: u32 = 8;
const CONTROL_MAX_CLOSE_LAYERS: u32 = 4;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct CoolingLimits {
    max_layer_index: u32,
    max_close_layers: u32,
}

fn live_limits(run: &PropertyRun) -> CoolingLimits {
    let mut bounds = BTreeMap::new();
    for key in BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("H-COOLING bound `{key}`: {error}"));
        bounds.insert(key, value);
    }
    CoolingLimits {
        max_layer_index: u32::try_from(bounds["max_layer_index"])
            .expect("layer-index bound fits in u32"),
        max_close_layers: u32::try_from(bounds["max_close_layers"])
            .expect("close-layer bound fits in u32"),
    }
}

fn control_limits() -> CoolingLimits {
    CoolingLimits {
        max_layer_index: CONTROL_MAX_LAYER_INDEX,
        max_close_layers: CONTROL_MAX_CLOSE_LAYERS,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Domain
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FanMode {
    Zero,
    Max,
    Random,
}

const FAN_MODES: [FanMode; 3] = [FanMode::Zero, FanMode::Max, FanMode::Random];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundaryMode {
    Before,
    At,
    After,
    Random,
}

const BOUNDARY_MODES: [BoundaryMode; 4] = [
    BoundaryMode::Before,
    BoundaryMode::At,
    BoundaryMode::After,
    BoundaryMode::Random,
];

/// Entity-role shapes. Selected with `prop::sample::select(...).no_shrink()` in
/// the property, so shrinking can remove *extra* roles but never flips a
/// witness between "has a `BridgeInfill`" and "has none".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoleMode {
    Empty,
    /// Exactly one `BridgeInfill` entity.
    SingleOverhang,
    /// Exactly one non-overhang entity.
    SinglePlain,
    /// A `BridgeInfill` surrounded by bounded plain roles.
    OverhangAndPlain,
    /// One to three roles drawn from the full role set.
    MultiRole,
}

const ROLE_MODES: [RoleMode; 5] = [
    RoleMode::Empty,
    RoleMode::SingleOverhang,
    RoleMode::SinglePlain,
    RoleMode::OverhangAndPlain,
    RoleMode::MultiRole,
];

const PLAIN_ROLES: [ExtrusionRole; 4] = [
    ExtrusionRole::OuterWall,
    ExtrusionRole::InnerWall,
    ExtrusionRole::SparseInfill,
    ExtrusionRole::Skirt,
];

const FULL_ROLES: [ExtrusionRole; 6] = [
    ExtrusionRole::OuterWall,
    ExtrusionRole::InnerWall,
    ExtrusionRole::SparseInfill,
    ExtrusionRole::BridgeInfill,
    ExtrusionRole::InternalBridgeInfill,
    ExtrusionRole::Skirt,
];

#[derive(Debug, Clone)]
struct CoolingCase {
    fan_max_speed: u8,
    close_layers: u32,
    overhang_fan_speed: u8,
    enable_overhang_bridge_fan: bool,
    layer_index: u32,
    roles: Vec<ExtrusionRole>,
    fan_mode: FanMode,
    boundary_mode: BoundaryMode,
    role_mode: RoleMode,
}

fn roles_for(mode: RoleMode, seeds: &[u32; 3]) -> Vec<ExtrusionRole> {
    let count = (seeds[0] % 4) as usize;
    let extra: Vec<ExtrusionRole> = (0..count)
        .map(|index| {
            PLAIN_ROLES[((seeds[1] >> (index * 4)) & 0xff) as usize % PLAIN_ROLES.len()].clone()
        })
        .collect();
    match mode {
        RoleMode::Empty => Vec::new(),
        RoleMode::SingleOverhang => vec![ExtrusionRole::BridgeInfill],
        RoleMode::SinglePlain => {
            vec![PLAIN_ROLES[(seeds[1] & 0xff) as usize % PLAIN_ROLES.len()].clone()]
        }
        RoleMode::OverhangAndPlain => {
            let mut roles = extra;
            let position = roles.len() / 2;
            roles.insert(position, ExtrusionRole::BridgeInfill);
            roles
        }
        RoleMode::MultiRole => (0..count.max(1))
            .map(|index| {
                FULL_ROLES[((seeds[2] >> (index * 4)) & 0xff) as usize % FULL_ROLES.len()].clone()
            })
            .collect(),
    }
}

/// Construction-first case builder. Every numeric input is derived from the
/// integer seed by bounded arithmetic, so no case is ever rejected.
fn build_case(
    fan_mode: FanMode,
    boundary_mode: BoundaryMode,
    role_mode: RoleMode,
    enable_overhang_bridge_fan: bool,
    seed: [u32; 4],
    limits: &CoolingLimits,
) -> CoolingCase {
    let fan_max_speed = match fan_mode {
        FanMode::Zero => 0,
        FanMode::Max => u8::MAX,
        FanMode::Random => (seed[0] & 0xff) as u8,
    };
    let overhang_fan_speed = ((seed[0] >> 8) % 101) as u8;
    let close_layers = seed[1] % (limits.max_close_layers + 1);
    let raw_layer_index = seed[2] % (limits.max_layer_index + 1);
    let layer_index = match boundary_mode {
        BoundaryMode::Before => close_layers.saturating_sub(1),
        BoundaryMode::At => close_layers,
        // The registry profiles satisfy `max_close_layers < max_layer_index`;
        // the clamp only keeps a hypothetical profile inside its own declared
        // index bound and is never observed on the measured profiles.
        BoundaryMode::After => (close_layers + 1).min(limits.max_layer_index),
        BoundaryMode::Random => raw_layer_index,
    };
    let role_seeds = [seed[3], seed[3] >> 8, seed[3] >> 16];
    let roles = roles_for(role_mode, &role_seeds);

    CoolingCase {
        fan_max_speed,
        close_layers,
        overhang_fan_speed,
        enable_overhang_bridge_fan,
        layer_index,
        roles,
        fan_mode,
        boundary_mode,
        role_mode,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures through the real constructors
// ─────────────────────────────────────────────────────────────────────────────

fn config_for(case: &CoolingCase) -> ConfigView {
    ConfigView::from_map(HashMap::from([
        (
            "fan_max_speed".to_string(),
            ConfigValue::Int(i64::from(case.fan_max_speed)),
        ),
        (
            "close_fan_the_first_x_layers".to_string(),
            ConfigValue::Int(i64::from(case.close_layers)),
        ),
        (
            "enable_overhang_bridge_fan".to_string(),
            ConfigValue::Bool(case.enable_overhang_bridge_fan),
        ),
        (
            "overhang_fan_speed".to_string(),
            ConfigValue::Int(i64::from(case.overhang_fan_speed)),
        ),
    ]))
}

fn module_for(case: &CoolingCase) -> Result<PartCooling, String> {
    let config = config_for(case);
    PartCooling::from_config(&config).map_err(|error| format!("from_config: {}", error.message))
}

fn view_for(case: &CoolingCase) -> LayerCollectionView {
    let entities: Vec<PrintEntity> = case
        .roles
        .iter()
        .enumerate()
        .map(|(index, role)| PrintEntity {
            entity_id: index as u64 + 1,
            path: extrusion_path3d_base(role.clone()),
            ..print_entity_base(role.clone())
        })
        .collect();
    LayerCollectionView::new(LayerCollectionIR {
        global_layer_index: case.layer_index,
        ordered_entities: entities,
        ..Default::default()
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent decision table
// ─────────────────────────────────────────────────────────────────────────────

/// Mutation knobs for the controls. The property always passes
/// `CoolingModel::default()`.
#[derive(Debug, Clone, Copy, Default)]
struct CoolingModel {
    /// Treat the close boundary as `layer_index <= close_layers`.
    strict_boundary: bool,
    /// Compute the overhang percentage from `overhang_fan_speed` instead of
    /// `fan_max_speed`.
    percent_base_is_overhang: bool,
    /// Apply the overhang bump even when the feature is disabled.
    bump_when_disabled: bool,
    /// Treat `InternalBridgeInfill` as an overhang role too.
    internal_bridge_is_overhang: bool,
}

fn is_overhang_role(role: &ExtrusionRole, model: &CoolingModel) -> bool {
    match role {
        ExtrusionRole::BridgeInfill => true,
        ExtrusionRole::InternalBridgeInfill => model.internal_bridge_is_overhang,
        _ => false,
    }
}

/// Returns `(base, result)`, both exact integers.
fn expected_table(case: &CoolingCase, model: &CoolingModel) -> (u8, u8) {
    let in_close_band = if model.strict_boundary {
        case.layer_index <= case.close_layers
    } else {
        case.layer_index < case.close_layers
    };
    let base = if in_close_band { 0 } else { case.fan_max_speed };

    let percentage_base = if model.percent_base_is_overhang {
        u16::from(case.overhang_fan_speed)
    } else {
        u16::from(case.fan_max_speed)
    };
    let overhang = (u16::from(case.overhang_fan_speed) * percentage_base) / 100;
    // The declared domain (`overhang_fan_speed <= 100`, `fan_max_speed <= 255`)
    // bounds the quotient at 255, so this conversion is exact.
    let overhang = u8::try_from(overhang).expect("declared ranges bound the overhang rate to 255");

    let has_overhang = case.roles.iter().any(|role| is_overhang_role(role, model));
    let bumped = has_overhang && (case.enable_overhang_bridge_fan || model.bump_when_disabled);
    (base, if bumped { overhang } else { base })
}

fn check_cooling_case(case: &CoolingCase, model: &CoolingModel) -> Result<(), String> {
    let module = module_for(case)?;
    let (expected_base, expected_result) = expected_table(case, model);

    let actual_base = module.layer_fan_speed(case.layer_index);
    if actual_base != expected_base {
        return Err(format!(
            "layer_fan_speed({}) on {case:?} returned {actual_base}, expected {expected_base}",
            case.layer_index
        ));
    }

    let view = view_for(case);
    let actual = module.cooling_decision_for_event(&view);
    if actual != expected_result {
        return Err(format!(
            "cooling_decision_for_event on {case:?} returned {actual}, expected {expected_result}"
        ));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy
// ─────────────────────────────────────────────────────────────────────────────

/// The shrinkable part of a case: the feature flag (fixed through shrinking)
/// plus four integer seeds that `build_case` maps into the declared domain.
type CoreSeed = (bool, [u32; 4]);

fn core_strategy() -> impl Strategy<Value = CoreSeed> {
    (
        prop::sample::select(vec![false, true]).no_shrink(),
        any::<[u32; 4]>(),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_cooling_decision_table() {
    let run = PropertyRun::new(PROPERTY_ID, PROPERTY_TEST).expect("H-COOLING context");
    let limits = live_limits(&run);

    let strategy = (
        prop::sample::select(FAN_MODES.to_vec()).no_shrink(),
        prop::sample::select(BOUNDARY_MODES.to_vec()).no_shrink(),
        prop::sample::select(ROLE_MODES.to_vec()).no_shrink(),
        core_strategy(),
    )
        .prop_map(
            move |(fan_mode, boundary_mode, role_mode, (enable, seed))| {
                build_case(fan_mode, boundary_mode, role_mode, enable, seed, &limits)
            },
        );

    let cases = Cell::new(0u64);
    let at_boundary = Cell::new(0u64);
    let before_boundary = Cell::new(0u64);
    let zero_fan = Cell::new(0u64);
    let empty_roles = Cell::new(0u64);
    let overhang_bumps = Cell::new(0u64);
    let disabled_with_overhang = Cell::new(0u64);

    run.check(&strategy, |case, _guard| {
        cases.set(cases.get() + 1);

        if let Err(message) = check_cooling_case(&case, &CoolingModel::default()) {
            return fail(message);
        }

        if case.layer_index == case.close_layers {
            at_boundary.set(at_boundary.get() + 1);
        }
        if case.close_layers > 0 && case.layer_index + 1 == case.close_layers {
            before_boundary.set(before_boundary.get() + 1);
        }
        if case.fan_max_speed == 0 {
            zero_fan.set(zero_fan.get() + 1);
        }
        if case.roles.is_empty() {
            empty_roles.set(empty_roles.get() + 1);
        }
        let has_overhang = case
            .roles
            .iter()
            .any(|role| matches!(role, ExtrusionRole::BridgeInfill));
        if has_overhang {
            if case.enable_overhang_bridge_fan {
                overhang_bumps.set(overhang_bumps.get() + 1);
            } else {
                disabled_with_overhang.set(disabled_with_overhang.get() + 1);
            }
        }
        Ok(())
    })
    .expect("H-COOLING layer/overhang decision table");

    assert!(cases.get() > 0, "no cooling case was generated");
    assert!(
        at_boundary.get() > 0,
        "no case sat exactly on the close-fan boundary"
    );
    assert!(
        before_boundary.get() > 0,
        "no case sat just below the close-fan boundary"
    );
    assert!(zero_fan.get() > 0, "no case used fan_max_speed = 0");
    assert!(empty_roles.get() > 0, "no case used an empty role set");
    assert!(
        overhang_bumps.get() > 0,
        "no enabled case carried a BridgeInfill overhang role"
    );
    assert!(
        disabled_with_overhang.get() > 0,
        "no disabled case carried a BridgeInfill overhang role"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic controls and shrink probe
// ─────────────────────────────────────────────────────────────────────────────

fn control_case(
    roles: Vec<ExtrusionRole>,
    fan_max_speed: u8,
    close_layers: u32,
    overhang_fan_speed: u8,
    enable_overhang_bridge_fan: bool,
    layer_index: u32,
    boundary_mode: BoundaryMode,
    role_mode: RoleMode,
) -> CoolingCase {
    CoolingCase {
        fan_max_speed,
        close_layers,
        overhang_fan_speed,
        enable_overhang_bridge_fan,
        layer_index,
        roles,
        fan_mode: FanMode::Random,
        boundary_mode,
        role_mode,
    }
}

#[test]
fn cooling_controls() {
    let honest = CoolingModel::default();

    // 1. The close band is `layer_index < close_layers`, not `<=`.
    let boundary = control_case(
        Vec::new(),
        200,
        2,
        0,
        false,
        2,
        BoundaryMode::At,
        RoleMode::Empty,
    );
    check_cooling_case(&boundary, &honest).expect("the boundary layer is already cooling");
    let error = check_cooling_case(
        &boundary,
        &CoolingModel {
            strict_boundary: true,
            ..Default::default()
        },
    )
    .expect_err("a strict `<=` close boundary must be rejected");
    assert!(error.contains("layer_fan_speed"), "{error}");

    // 2. The overhang percentage is taken from `fan_max_speed`.
    let percentage = control_case(
        vec![ExtrusionRole::BridgeInfill],
        200,
        0,
        50,
        true,
        3,
        BoundaryMode::Random,
        RoleMode::SingleOverhang,
    );
    check_cooling_case(&percentage, &honest).expect("50% of 200 is 100");
    let error = check_cooling_case(
        &percentage,
        &CoolingModel {
            percent_base_is_overhang: true,
            ..Default::default()
        },
    )
    .expect_err("a percentage taken from the wrong base must be rejected");
    assert!(error.contains("cooling_decision_for_event"), "{error}");

    // 3. A disabled feature never applies the overhang bump.
    let disabled = CoolingCase {
        enable_overhang_bridge_fan: false,
        ..percentage.clone()
    };
    check_cooling_case(&disabled, &honest).expect("a disabled feature keeps the base speed");
    let error = check_cooling_case(
        &disabled,
        &CoolingModel {
            bump_when_disabled: true,
            ..Default::default()
        },
    )
    .expect_err("an overhang bump with the feature disabled must be rejected");
    assert!(error.contains("cooling_decision_for_event"), "{error}");

    // 4. Only `BridgeInfill` is an overhang role.
    let role_set = control_case(
        vec![ExtrusionRole::InternalBridgeInfill],
        200,
        0,
        50,
        true,
        3,
        BoundaryMode::Random,
        RoleMode::MultiRole,
    );
    check_cooling_case(&role_set, &honest).expect("InternalBridgeInfill is not an overhang role");
    let error = check_cooling_case(
        &role_set,
        &CoolingModel {
            internal_bridge_is_overhang: true,
            ..Default::default()
        },
    )
    .expect_err("an over-wide overhang role set must be rejected");
    assert!(error.contains("cooling_decision_for_event"), "{error}");

    cooling_witness_probe();
}

fn check_witness(case: &CoolingCase, role_mode: RoleMode, enable: bool) {
    assert_eq!(case.role_mode, role_mode, "the role mode changed");
    assert_eq!(case.fan_mode, FanMode::Random, "the fan mode changed");
    assert_eq!(
        case.boundary_mode,
        BoundaryMode::At,
        "the boundary mode changed"
    );
    assert_eq!(
        case.layer_index, case.close_layers,
        "the boundary witness moved off the close boundary"
    );
    assert_eq!(
        case.enable_overhang_bridge_fan, enable,
        "the enable mode changed"
    );
    let has_bridge = case
        .roles
        .iter()
        .any(|role| matches!(role, ExtrusionRole::BridgeInfill));
    match role_mode {
        RoleMode::Empty | RoleMode::SinglePlain => {
            assert!(!has_bridge, "a {role_mode:?} witness gained a BridgeInfill");
        }
        RoleMode::SingleOverhang | RoleMode::OverhangAndPlain => {
            assert!(has_bridge, "a {role_mode:?} witness lost its BridgeInfill");
        }
        RoleMode::MultiRole => {}
    }
    check_cooling_case(case, &CoolingModel::default()).expect("the witness case checks");
}

fn cooling_witness_probe() {
    let limits = control_limits();

    for role_mode in ROLE_MODES {
        for enable in [false, true] {
            let strategy = any::<[u32; 4]>().prop_map(move |seed| {
                build_case(
                    FanMode::Random,
                    BoundaryMode::At,
                    role_mode,
                    enable,
                    seed,
                    &limits,
                )
            });
            let mut runner = TestRunner::deterministic();
            let mut tree = strategy
                .new_tree(&mut runner)
                .expect("cooling witness tree");

            check_witness(&tree.current(), role_mode, enable);
            let initial = format!("{:?}", tree.current());
            let mut simplified = 0usize;
            let mut changed = false;
            while tree.simplify() {
                simplified += 1;
                let current = tree.current();
                check_witness(&current, role_mode, enable);
                changed |= format!("{current:?}") != initial;
                assert!(
                    tree.complicate(),
                    "complicate must restore the last simplification"
                );
                check_witness(&tree.current(), role_mode, enable);
            }
            assert!(
                simplified > 0,
                "false-only shrink probe for the {role_mode:?}/enable={enable} witness"
            );
            assert!(
                changed,
                "no allowed reduction changed the {role_mode:?}/enable={enable} witness from its nonminimal start"
            );
            println!(
                "role_mode={role_mode:?} enable={enable} simplify={simplified} changed={changed}"
            );
        }
    }

    // The role-mode, fan-mode and boundary-mode selectors must never shrink: a
    // shrinking mode tree could switch a witness's class mid-search.
    assert_mode_selector_does_not_shrink(ROLE_MODES.to_vec());
    assert_mode_selector_does_not_shrink(FAN_MODES.to_vec());
    assert_mode_selector_does_not_shrink(BOUNDARY_MODES.to_vec());
}

fn assert_mode_selector_does_not_shrink<T>(modes: Vec<T>)
where
    T: Clone + std::fmt::Debug + 'static,
{
    let mut runner = TestRunner::deterministic();
    let strategy = prop::sample::select(modes).no_shrink();
    let mut tree = strategy.new_tree(&mut runner).expect("mode-selector tree");
    let before = format!("{:?}", tree.current());
    assert!(!tree.simplify(), "the mode selector simplified");
    assert!(!tree.complicate(), "the mode selector complicated");
    assert_eq!(format!("{:?}", tree.current()), before);
}
