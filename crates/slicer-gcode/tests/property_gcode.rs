//! Opt-in property tests for `slicer-gcode`'s feedrate, emission and
//! serializer seams (packet `proptest-openrscad_16_pbt-io-gcode`, charters
//! `H-GCODE-FEEDRATE`, `H-GCODE-EMIT` and `H-GCODE-SERIALIZE`). The generated
//! properties live here; deterministic controls share the same checkers.
//!
//! Gated by the `property_testing` selection feature through the manifest
//! `[[test]]` entry, and skipped entirely by an ordinary build.
//!
//! The generative bodies are registered properties: each one reads its exact
//! name and bounds from the registry context (`PNP_PROPERTY_CONTEXT`) through
//! `PropertyRun`, so a plain `cargo test` of a property is invalid by design.
//! The `*_controls` tests are plain deterministic tests and never construct a
//! `PropertyRun`.

#![allow(missing_docs)]

use proptest::prelude::*;
use proptest::sample::select;
use proptest::strategy::ValueTree;
use proptest::test_runner::{RngSeed, TestCaseError, TestRunner};

use slicer_gcode::emit::DefaultGCodeEmitter;
use slicer_gcode::{format_coord, format_xyz};
use slicer_gcode::{DefaultGCodeSerializer, GCodeEmitter, GCodeSerializer, GcodeFlavor};
use slicer_ir::{ExtrusionRole, FeedrateConfig, GCodeCommand, GCodeIR};
use slicer_sdk::test_support::property_testing::PropertyRun;

// ════════════════════════════════════════════════════════════════════════════
// Shared helpers
// ════════════════════════════════════════════════════════════════════════════

/// Render an integer count of `10^-decimals` units with trailing zeros removed.
///
/// This is the independent decimal model used by the serializer oracle. On the
/// declared dyadic domains every scaled value is an exact integer, so
/// round-to-nearest cannot disagree with `core::fmt`; the tie and signed-zero
/// behaviour is pinned separately by `serializer_formatting_probe` from the
/// real `format_xyz`/`format_coord` output.
fn format_scaled_trimmed(scaled: i128, decimals: u32) -> String {
    let negative = scaled < 0;
    let magnitude = scaled.unsigned_abs();
    let scale = 10_u128.pow(decimals);
    let whole = magnitude / scale;
    let mut fraction = magnitude % scale;
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&whole.to_string());
    if fraction != 0 && decimals > 0 {
        let mut digits = String::with_capacity(decimals as usize);
        for _ in 0..decimals {
            digits.push(char::from(b'0' + (fraction % 10) as u8));
            fraction /= 10;
        }
        let digits: String = digits.chars().rev().collect();
        let visible = digits.trim_end_matches('0');
        if !visible.is_empty() {
            out.push('.');
            out.push_str(visible);
        }
    }
    out
}

/// Render an integer count of `10^-decimals` units with exactly `decimals`
/// fractional digits (the `{:.N}` wire shape used by E tokens).
fn format_scaled_fixed(scaled: i128, decimals: u32) -> String {
    let negative = scaled < 0;
    let magnitude = scaled.unsigned_abs();
    let scale = 10_u128.pow(decimals);
    let whole = magnitude / scale;
    let fraction = magnitude % scale;
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&whole.to_string());
    if decimals > 0 {
        out.push('.');
        out.push_str(&format!("{:0width$}", fraction, width = decimals as usize));
    }
    out
}

/// Independent trimmed decimal rendering of one f32 coordinate.
///
/// On the declared domain (`1/8` mm XY, `1/4` mm Z) the value times
/// `10^decimals` is an exact integer, so this agrees with the production
/// formatter's fast path. The production formatter's own tie and `-0` behaviour
/// is measured by [`serializer_formatting_probe`] before this model is trusted.
fn oracle_coord(value: f32, decimals: u32) -> String {
    let scale = 10_f64.powi(decimals as i32);
    let scaled = (f64::from(value) * scale).round() as i128;
    let mut rendered = format_scaled_trimmed(scaled, decimals);
    if value == 0.0 && value.is_sign_negative() {
        rendered.insert(0, '-');
    }
    rendered
}

/// Independent fixed 5-decimal rendering of one f64 extrusion value.
fn oracle_e5(value: f64) -> String {
    let scaled = (value * 100_000.0).round() as i128;
    if value == 0.0 && value.is_sign_negative() {
        return "-0.00000".to_string();
    }
    format_scaled_fixed(scaled, 5)
}

/// ADR-0052 conversion: `round(base * 60 * clamp(factor, 0.05, 5.0) * 1000) / 1000`.
///
/// The multiplication runs in f64 and Rust's `f64::round` is half-away-from-zero.
/// This is the documented contract, not a call into the production seam.
fn feedrate_from_base_mm_per_s(base_mm_s: f32, factor: f32) -> f32 {
    let clamped = factor.clamp(0.05, 5.0);
    let product = f64::from(base_mm_s) * 60.0 * f64::from(clamped);
    ((product * 1000.0).round() / 1000.0) as f32
}

/// Read a required positive registry bound.
fn positive_bound(run: &PropertyRun, name: &str) -> Result<u64, String> {
    let value = run
        .bound(name)
        .map_err(|error| format!("bound {name}: {error}"))?;
    if value == 0 {
        return Err(format!("bound {name} must be positive"));
    }
    Ok(value)
}

/// Deterministic `simplify`/`complicate` witness probe shared by every charter.
///
/// The probe runs two deterministic passes over the same fixed seed: the first
/// counts the accepted `simplify()` steps to exhaustion, the second replays
/// exactly those accepted steps (never the final failed attempt, which still
/// mutates Proptest's binary-search integer trees) and then calls `complicate`
/// while asserting every restored state is still a valid witness with its
/// persistent mode intact. It follows Proptest's actual `ValueTree` protocol and
/// never claims that `complicate` can replay a multi-component history it does
/// not expose. Fixed seeds keep the probe reproducible in a control run.
fn shrink_complicate_witness<S>(
    label: &str,
    strategy: &S,
    mode_of: impl Fn(&S::Value) -> Option<String>,
    check_valid: impl Fn(&S::Value) -> Result<(), String>,
) where
    S: Strategy,
    S::Value: Clone + PartialEq + std::fmt::Debug,
{
    let build_tree = |seed: u64| {
        let mut runner = TestRunner::new(ProptestConfig {
            cases: 1,
            rng_seed: RngSeed::Fixed(seed),
            failure_persistence: None,
            ..ProptestConfig::default()
        });
        strategy.new_tree(&mut runner)
    };

    for seed in 0..64_u64 {
        let mut counter = match build_tree(seed) {
            Ok(tree) => tree,
            Err(error) => {
                panic!("{label} witness probe seed {seed} could not build a tree: {error}")
            }
        };
        let mut simplifications = 0_usize;
        while counter.simplify() {
            simplifications += 1;
        }
        if simplifications == 0 {
            continue;
        }

        let mut tree = match build_tree(seed) {
            Ok(tree) => tree,
            Err(error) => {
                panic!("{label} witness probe seed {seed} could not rebuild its tree: {error}")
            }
        };
        let start = tree.current().clone();
        let start_mode = mode_of(&start);
        if let Err(error) = check_valid(&start) {
            panic!("{label} witness probe seed {seed} produced an invalid start state: {error}");
        }
        for step in 1..=simplifications {
            assert!(
                tree.simplify(),
                "{label} witness probe seed {seed}: the deterministic replay diverged at simplify step {step}"
            );
            let state = tree.current().clone();
            if let Err(error) = check_valid(&state) {
                panic!(
                    "{label} witness probe seed {seed} simplified into an invalid state (step {step}): {error}"
                );
            }
            let mode = mode_of(&state);
            if let (Some(mode), Some(expected)) = (mode, start_mode.as_ref()) {
                assert_eq!(
                    mode, *expected,
                    "{label} witness probe seed {seed}: simplify must preserve the persistent mode"
                );
            }
        }

        let mut restored = 0_usize;
        while tree.complicate() {
            restored += 1;
            let state = tree.current().clone();
            if let Err(error) = check_valid(&state) {
                panic!(
                    "{label} witness probe seed {seed} complicated into an invalid state (step {restored}): {error}"
                );
            }
            let mode = mode_of(&state);
            if let (Some(mode), Some(expected)) = (mode, start_mode.as_ref()) {
                assert_eq!(
                    mode, *expected,
                    "{label} witness probe seed {seed}: complicate must preserve the persistent mode"
                );
            }
        }
        assert!(
            restored >= 1,
            "{label} witness probe seed {seed}: the protocol must expose at least one complicate step after {simplifications} simplifications"
        );
        assert!(
            !tree.complicate(),
            "{label} witness probe seed {seed}: an exhausted history must keep reporting false"
        );
        let mode = mode_of(&tree.current().clone());
        if let (Some(mode), Some(expected)) = (mode, start_mode.as_ref()) {
            assert_eq!(
                mode, *expected,
                "{label} witness probe seed {seed}: the final restored state must keep its persistent mode"
            );
        }
        return;
    }
    panic!("{label} witness probe did not obtain a nonminimal value tree from any of the first 64 fixed seeds");
}

// ════════════════════════════════════════════════════════════════════════════
// H-GCODE-FEEDRATE: role table, factor clamp, conversion and rounding
// ════════════════════════════════════════════════════════════════════════════

/// Registry identity of the feedrate charter.
const FEEDRATE_PROPERTY_ID: &str = "H-GCODE-FEEDRATE";
/// Exact registry test name of the feedrate charter.
const FEEDRATE_TEST_NAME: &str = "proptest_feedrate_resolution";

/// One `ExtrusionRole` variant plus the three documented custom names.
///
/// `role()` maps every slot onto the real IR value; the oracle switches on the
/// slot, never on the production `role_base_speed_mm_per_s` match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoleSlot {
    OuterWall,
    InnerWall,
    ThinWall,
    TopSolidInfill,
    BottomSolidInfill,
    InternalSolidInfill,
    SparseInfill,
    RaftInfill,
    SupportMaterial,
    SupportInterface,
    SupportBaseInterface,
    WipeTower,
    PrimeTower,
    Ironing,
    BridgeInfill,
    InternalBridgeInfill,
    Skirt,
    Brim,
    GapFill,
    CustomTravel,
    CustomWipe,
    CustomGapInfill,
    CustomOther,
}

impl RoleSlot {
    /// Every slot, including one per real `ExtrusionRole` variant.
    const ALL: [RoleSlot; 23] = [
        RoleSlot::OuterWall,
        RoleSlot::InnerWall,
        RoleSlot::ThinWall,
        RoleSlot::TopSolidInfill,
        RoleSlot::BottomSolidInfill,
        RoleSlot::InternalSolidInfill,
        RoleSlot::SparseInfill,
        RoleSlot::RaftInfill,
        RoleSlot::SupportMaterial,
        RoleSlot::SupportInterface,
        RoleSlot::SupportBaseInterface,
        RoleSlot::WipeTower,
        RoleSlot::PrimeTower,
        RoleSlot::Ironing,
        RoleSlot::BridgeInfill,
        RoleSlot::InternalBridgeInfill,
        RoleSlot::Skirt,
        RoleSlot::Brim,
        RoleSlot::GapFill,
        RoleSlot::CustomTravel,
        RoleSlot::CustomWipe,
        RoleSlot::CustomGapInfill,
        RoleSlot::CustomOther,
    ];

    /// The real IR role this slot denotes.
    fn role(self) -> ExtrusionRole {
        match self {
            RoleSlot::OuterWall => ExtrusionRole::OuterWall,
            RoleSlot::InnerWall => ExtrusionRole::InnerWall,
            RoleSlot::ThinWall => ExtrusionRole::ThinWall,
            RoleSlot::TopSolidInfill => ExtrusionRole::TopSolidInfill,
            RoleSlot::BottomSolidInfill => ExtrusionRole::BottomSolidInfill,
            RoleSlot::InternalSolidInfill => ExtrusionRole::InternalSolidInfill,
            RoleSlot::SparseInfill => ExtrusionRole::SparseInfill,
            RoleSlot::RaftInfill => ExtrusionRole::RaftInfill,
            RoleSlot::SupportMaterial => ExtrusionRole::SupportMaterial,
            RoleSlot::SupportInterface => ExtrusionRole::SupportInterface,
            RoleSlot::SupportBaseInterface => ExtrusionRole::SupportBaseInterface,
            RoleSlot::WipeTower => ExtrusionRole::WipeTower,
            RoleSlot::PrimeTower => ExtrusionRole::PrimeTower,
            RoleSlot::Ironing => ExtrusionRole::Ironing,
            RoleSlot::BridgeInfill => ExtrusionRole::BridgeInfill,
            RoleSlot::InternalBridgeInfill => ExtrusionRole::InternalBridgeInfill,
            RoleSlot::Skirt => ExtrusionRole::Skirt,
            RoleSlot::Brim => ExtrusionRole::Brim,
            RoleSlot::GapFill => ExtrusionRole::GapFill,
            RoleSlot::CustomTravel => ExtrusionRole::Custom("Travel".to_string()),
            RoleSlot::CustomWipe => ExtrusionRole::Custom("Wipe".to_string()),
            RoleSlot::CustomGapInfill => ExtrusionRole::Custom("GapInfill".to_string()),
            RoleSlot::CustomOther => ExtrusionRole::Custom("PropertyOther".to_string()),
        }
    }

    /// Classify a real role back onto the documented decision table.
    fn from_role(role: &ExtrusionRole) -> RoleSlot {
        match role {
            ExtrusionRole::OuterWall => RoleSlot::OuterWall,
            ExtrusionRole::InnerWall => RoleSlot::InnerWall,
            ExtrusionRole::ThinWall => RoleSlot::ThinWall,
            ExtrusionRole::TopSolidInfill => RoleSlot::TopSolidInfill,
            ExtrusionRole::BottomSolidInfill => RoleSlot::BottomSolidInfill,
            ExtrusionRole::InternalSolidInfill => RoleSlot::InternalSolidInfill,
            ExtrusionRole::SparseInfill => RoleSlot::SparseInfill,
            ExtrusionRole::RaftInfill => RoleSlot::RaftInfill,
            ExtrusionRole::SupportMaterial => RoleSlot::SupportMaterial,
            ExtrusionRole::SupportInterface => RoleSlot::SupportInterface,
            ExtrusionRole::SupportBaseInterface => RoleSlot::SupportBaseInterface,
            ExtrusionRole::WipeTower => RoleSlot::WipeTower,
            ExtrusionRole::PrimeTower => RoleSlot::PrimeTower,
            ExtrusionRole::Ironing => RoleSlot::Ironing,
            ExtrusionRole::BridgeInfill => RoleSlot::BridgeInfill,
            ExtrusionRole::InternalBridgeInfill => RoleSlot::InternalBridgeInfill,
            ExtrusionRole::Skirt => RoleSlot::Skirt,
            ExtrusionRole::Brim => RoleSlot::Brim,
            ExtrusionRole::GapFill => RoleSlot::GapFill,
            ExtrusionRole::Custom(name) => match name.as_str() {
                "Travel" => RoleSlot::CustomTravel,
                "Wipe" => RoleSlot::CustomWipe,
                "GapInfill" => RoleSlot::CustomGapInfill,
                _ => RoleSlot::CustomOther,
            },
        }
    }
}

/// The independent role decision table from the charter.
fn expected_base_speed_mm_per_s(config: &FeedrateConfig, slot: RoleSlot) -> f32 {
    match slot {
        RoleSlot::OuterWall => config.outer_wall_speed,
        RoleSlot::InnerWall => config.inner_wall_speed,
        RoleSlot::ThinWall => config.thin_wall_speed,
        RoleSlot::TopSolidInfill => config.top_surface_speed,
        RoleSlot::BottomSolidInfill => config.bottom_surface_speed,
        RoleSlot::InternalSolidInfill | RoleSlot::SparseInfill => config.sparse_infill_speed,
        RoleSlot::BridgeInfill => config.bridge_speed,
        RoleSlot::InternalBridgeInfill => {
            let value = config.internal_bridge_speed;
            if value.is_percent {
                // Charter: percent mode is `(value / 100) * bridge_speed`.
                value.value as f32 / 100.0 * config.bridge_speed
            } else {
                value.value as f32
            }
        }
        RoleSlot::SupportMaterial => config.support_speed,
        RoleSlot::SupportInterface | RoleSlot::SupportBaseInterface => {
            config.support_interface_speed
        }
        RoleSlot::Skirt | RoleSlot::Brim => config.skirt_speed,
        RoleSlot::WipeTower => config
            .wipe_tower_max_purge_speed
            .min(config.sparse_infill_speed),
        RoleSlot::PrimeTower => config.prime_tower_speed,
        RoleSlot::GapFill => config.gap_infill_speed,
        RoleSlot::RaftInfill => config.outer_wall_speed,
        RoleSlot::Ironing => {
            if config.filament_ironing_speed > 0.0 {
                config.filament_ironing_speed
            } else {
                config.ironing_speed
            }
        }
        RoleSlot::CustomTravel => config.travel_speed,
        RoleSlot::CustomWipe => config.wipe_speed,
        RoleSlot::CustomGapInfill => config.gap_infill_speed,
        RoleSlot::CustomOther => config.outer_wall_speed,
    }
}

/// Persistent factor mode; shrinking must retain it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactorMode {
    Zero,
    Negative,
    BelowLow,
    ExactLow,
    Inside,
    ExactHigh,
    AboveHigh,
}

impl FactorMode {
    const ALL: [FactorMode; 7] = [
        FactorMode::Zero,
        FactorMode::Negative,
        FactorMode::BelowLow,
        FactorMode::ExactLow,
        FactorMode::Inside,
        FactorMode::ExactHigh,
        FactorMode::AboveHigh,
    ];

    /// Clamp-relative classification of a factor, used to prove the mode held.
    fn classify(value: f32) -> &'static str {
        if value == 0.0 {
            "zero"
        } else if value < 0.0 {
            "negative"
        } else if value < 0.05 {
            "below-low"
        } else if value == 0.05 {
            "exact-low"
        } else if value < 5.0 {
            "inside"
        } else if value == 5.0 {
            "exact-high"
        } else {
            "above-high"
        }
    }
}

/// One generated feedrate case: a role slot, a base speed in milli-mm/s, a
/// persistent factor mode plus magnitude, and the internal-bridge/ironing mode
/// bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FeedrateCase {
    slot: RoleSlot,
    base_milli: u64,
    factor_mode: FactorMode,
    factor_magnitude: u64,
    bridge_percent: bool,
    ironing_filament_override: bool,
}

impl FeedrateCase {
    fn base_mm_s(&self) -> f32 {
        self.base_milli as f32 / 1000.0
    }

    fn factor_milli(&self) -> i32 {
        let magnitude = i32::try_from(self.factor_magnitude).unwrap_or(i32::MAX);
        match self.factor_mode {
            FactorMode::Zero => 0,
            FactorMode::Negative => -magnitude.max(1),
            FactorMode::BelowLow => 1 + magnitude % 49,
            FactorMode::ExactLow => 50,
            FactorMode::Inside => 51 + magnitude % 4_949,
            FactorMode::ExactHigh => 5_000,
            FactorMode::AboveHigh => 5_001 + magnitude % 5_000,
        }
    }

    fn factor(&self) -> f32 {
        self.factor_milli() as f32 / 1000.0
    }
}

/// Checked registry bounds for the feedrate charter.
#[derive(Debug, Clone, Copy)]
struct FeedrateBounds {
    max_base_milli: u64,
    max_factor_milli: u64,
}

/// Test-local profile mirroring the registry's measured routine bounds.
const CONTROL_FEEDRATE_BOUNDS: FeedrateBounds = FeedrateBounds {
    max_base_milli: 1_000_000,
    max_factor_milli: 10_000,
};

fn feedrate_bounds(run: &PropertyRun) -> Result<FeedrateBounds, String> {
    let max_base_speed_mm_s = positive_bound(run, "max_base_speed_mm_s")?;
    let max_factor_milli = positive_bound(run, "max_factor_milli")?;
    if max_factor_milli <= 5_000 {
        return Err(format!(
            "bound max_factor_milli must exceed 5000 to admit the above-clamp witness, got {max_factor_milli}"
        ));
    }
    let max_base_milli = max_base_speed_mm_s.checked_mul(1_000).ok_or_else(|| {
        format!("bound max_base_speed_mm_s {max_base_speed_mm_s} overflows milli-units")
    })?;
    Ok(FeedrateBounds {
        max_base_milli,
        max_factor_milli,
    })
}

/// Build the `FeedrateConfig` a generated case names. Every non-selected field
/// is a distinct nonzero constant so any role-table swap changes the answer.
fn feedrate_config_for_case(case: &FeedrateCase) -> FeedrateConfig {
    let mut config = FeedrateConfig {
        outer_wall_speed: 11.0,
        inner_wall_speed: 12.0,
        thin_wall_speed: 13.0,
        top_surface_speed: 14.0,
        bottom_surface_speed: 15.0,
        sparse_infill_speed: 16.0,
        bridge_speed: 17.0,
        internal_bridge_speed: slicer_ir::resolved_config::ResolvedFloatOrPercent {
            value: 18.0,
            is_percent: false,
        },
        support_speed: 19.0,
        support_interface_speed: 20.0,
        gap_infill_speed: 21.0,
        ironing_speed: 22.0,
        skirt_speed: 23.0,
        wipe_tower_max_purge_speed: 24.0,
        prime_tower_speed: 25.0,
        travel_speed: 26.0,
        wipe_speed: 27.0,
        filament_ironing_speed: 0.0,
        ..Default::default()
    };

    let base = case.base_mm_s();
    match case.slot {
        RoleSlot::OuterWall => config.outer_wall_speed = base,
        RoleSlot::InnerWall => config.inner_wall_speed = base,
        RoleSlot::ThinWall => config.thin_wall_speed = base,
        RoleSlot::TopSolidInfill => config.top_surface_speed = base,
        RoleSlot::BottomSolidInfill => config.bottom_surface_speed = base,
        RoleSlot::InternalSolidInfill | RoleSlot::SparseInfill => {
            config.sparse_infill_speed = base;
        }
        RoleSlot::RaftInfill => config.outer_wall_speed = base,
        RoleSlot::BridgeInfill => config.bridge_speed = base,
        RoleSlot::InternalBridgeInfill => {
            config.internal_bridge_speed = slicer_ir::resolved_config::ResolvedFloatOrPercent {
                value: case.base_milli as f64 / 1000.0,
                is_percent: case.bridge_percent,
            };
        }
        RoleSlot::SupportMaterial => config.support_speed = base,
        RoleSlot::SupportInterface | RoleSlot::SupportBaseInterface => {
            config.support_interface_speed = base;
        }
        RoleSlot::Skirt | RoleSlot::Brim => config.skirt_speed = base,
        RoleSlot::WipeTower => config.wipe_tower_max_purge_speed = base,
        RoleSlot::PrimeTower => config.prime_tower_speed = base,
        RoleSlot::GapFill => config.gap_infill_speed = base,
        RoleSlot::Ironing => {
            if case.ironing_filament_override {
                config.filament_ironing_speed = base;
                config.ironing_speed = 9.5;
            } else {
                config.filament_ironing_speed = 0.0;
                config.ironing_speed = base;
            }
        }
        RoleSlot::CustomTravel => config.travel_speed = base,
        RoleSlot::CustomWipe => config.wipe_speed = base,
        RoleSlot::CustomGapInfill => config.gap_infill_speed = base,
        RoleSlot::CustomOther => config.outer_wall_speed = base,
    }
    config
}

fn feedrate_case_strategy(bounds: FeedrateBounds) -> impl Strategy<Value = FeedrateCase> {
    (
        select(RoleSlot::ALL.to_vec()).no_shrink(),
        0..=bounds.max_base_milli,
        select(FactorMode::ALL.to_vec()).no_shrink(),
        0..=bounds.max_factor_milli,
        select(vec![false, true]).no_shrink(),
        select(vec![false, true]).no_shrink(),
    )
        .prop_map(
            |(
                slot,
                base_milli,
                factor_mode,
                factor_magnitude,
                bridge_percent,
                ironing_filament_override,
            )| {
                FeedrateCase {
                    slot,
                    base_milli,
                    factor_mode,
                    factor_magnitude,
                    bridge_percent,
                    ironing_filament_override,
                }
            },
        )
}

/// One named way the documented decision table can be wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeedrateMutation {
    /// The documented contract.
    None,
    /// Omit the mm/s → mm/min `× 60` conversion.
    MissingConversion,
    /// Clamp the factor to `[0, 1]` instead of `[0.05, 5.0]`.
    UnitClamp,
    /// Truncate the product at one millidegree instead of rounding.
    TruncateMillidegree,
    /// Swap the skirt and wipe-tower rows of the role table.
    RoleSwap,
    /// Treat the f64 product as the answer without round-to-millidegree.
    ExactRoundtrip,
}

fn expected_feedrate(
    config: &FeedrateConfig,
    slot: RoleSlot,
    factor: f32,
    mutation: FeedrateMutation,
) -> f32 {
    let base = match mutation {
        FeedrateMutation::RoleSwap => match slot {
            RoleSlot::Skirt => config
                .wipe_tower_max_purge_speed
                .min(config.sparse_infill_speed),
            RoleSlot::WipeTower => config.skirt_speed,
            _ => expected_base_speed_mm_per_s(config, slot),
        },
        _ => expected_base_speed_mm_per_s(config, slot),
    };
    match mutation {
        FeedrateMutation::MissingConversion => {
            let clamped = factor.clamp(0.05, 5.0);
            let product = f64::from(base) * f64::from(clamped);
            ((product * 1000.0).round() / 1000.0) as f32
        }
        FeedrateMutation::UnitClamp => {
            let clamped = factor.clamp(0.0, 1.0);
            let product = f64::from(base) * 60.0 * f64::from(clamped);
            ((product * 1000.0).round() / 1000.0) as f32
        }
        FeedrateMutation::TruncateMillidegree => {
            let clamped = factor.clamp(0.05, 5.0);
            let product = f64::from(base) * 60.0 * f64::from(clamped);
            ((product * 1000.0).trunc() / 1000.0) as f32
        }
        FeedrateMutation::ExactRoundtrip => {
            let clamped = factor.clamp(0.05, 5.0);
            (f64::from(base) * 60.0 * f64::from(clamped)) as f32
        }
        FeedrateMutation::None | FeedrateMutation::RoleSwap => {
            feedrate_from_base_mm_per_s(base, factor)
        }
    }
}

fn check_resolved_feedrate(
    config: &FeedrateConfig,
    slot: RoleSlot,
    factor: f32,
    mutation: FeedrateMutation,
) -> Result<(), String> {
    let emitter =
        DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), config.clone());
    let role = slot.role();
    let observed = emitter.resolve_feedrate(&role, factor);
    let expected = expected_feedrate(config, slot, factor, mutation);
    let observed = observed.ok_or_else(|| {
        format!("resolve_feedrate({role:?}, {factor}) returned None; the seam always returns Some")
    })?;
    if observed.to_bits() != expected.to_bits() {
        return Err(format!(
            "resolve_feedrate({role:?}, {factor}) returned {observed:?} (bits {:#010x}), the independent table requires {expected:?} (bits {:#010x})",
            observed.to_bits(),
            expected.to_bits()
        ));
    }
    Ok(())
}

fn check_feedrate_case(case: &FeedrateCase, mutation: FeedrateMutation) -> Result<(), String> {
    let expected_class = match case.factor_mode {
        FactorMode::Zero => "zero",
        FactorMode::Negative => "negative",
        FactorMode::BelowLow => "below-low",
        FactorMode::ExactLow => "exact-low",
        FactorMode::Inside => "inside",
        FactorMode::ExactHigh => "exact-high",
        FactorMode::AboveHigh => "above-high",
    };
    let observed_class = FactorMode::classify(case.factor());
    if observed_class != expected_class {
        return Err(format!(
            "generated factor {:?} is classified {observed_class}, but its persistent mode requires {expected_class}",
            case.factor()
        ));
    }
    let config = feedrate_config_for_case(case);
    check_resolved_feedrate(&config, case.slot, case.factor(), mutation)
}

/// Deterministic feedrate witnesses: every role at several factors, the exact
/// clamp endpoints, the zero base, the derived overflow boundary, and the
/// non-finite factor observations.
fn feedrate_deterministic_witnesses() -> Result<(), String> {
    // 1. Every role, several finite factors including both exact clamp ends.
    for slot in RoleSlot::ALL {
        let config = witness_feedrate_config(slot, 60.0);
        let base = expected_base_speed_mm_per_s(&config, slot);
        if base.to_bits() != 60.0_f32.to_bits() {
            return Err(format!(
                "witness fixture for {slot:?} selects base {base:?}, not the pinned 60.0"
            ));
        }
        for factor in [0.0_f32, 0.01, 0.05, 0.5, 1.0, 5.0, 7.5] {
            check_resolved_feedrate(&config, slot, factor, FeedrateMutation::None)?;
            let observed =
                DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), config.clone())
                    .resolve_feedrate(&slot.role(), factor);
            if observed.is_none() {
                return Err(format!("{slot:?} at factor {factor} returned None"));
            }
        }
    }

    // 2. Exact clamp endpoints: 0.049999 and 5.000001 must equal 0.05 and 5.0.
    let config = witness_feedrate_config(RoleSlot::OuterWall, 60.0);
    let emitter = DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), config);
    let low = emitter.resolve_feedrate(&ExtrusionRole::OuterWall, 0.049_999);
    let low_clamped = emitter.resolve_feedrate(&ExtrusionRole::OuterWall, 0.05);
    if low != low_clamped {
        return Err(format!(
            "factor 0.049999 must clamp to 0.05, got {low:?} vs {low_clamped:?}"
        ));
    }
    let high = emitter.resolve_feedrate(&ExtrusionRole::OuterWall, 5.000_001);
    let high_clamped = emitter.resolve_feedrate(&ExtrusionRole::OuterWall, 5.0);
    if high != high_clamped {
        return Err(format!(
            "factor 5.000001 must clamp to 5.0, got {high:?} vs {high_clamped:?}"
        ));
    }

    // 3. Zero selected base is Some(0.0), never an error or automatic speed.
    for slot in RoleSlot::ALL {
        let config = witness_feedrate_config(slot, 0.0);
        if expected_base_speed_mm_per_s(&config, slot) != 0.0 {
            return Err(format!(
                "zero-base witness for {slot:?} does not select a zero base"
            ));
        }
        let emitter = DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), config);
        for factor in [0.0_f32, 0.05, 1.0, 5.0, 10.0] {
            match emitter.resolve_feedrate(&slot.role(), factor) {
                Some(value) if value.to_bits() == 0.0_f32.to_bits() => {}
                other => {
                    return Err(format!(
                        "zero base for {slot:?} at factor {factor} must be Some(0.0), got {other:?}"
                    ))
                }
            }
        }
    }

    // 4. Derived overflow boundary: a finite base above f32::MAX / (60 * 5)
    //    narrows to +Infinity. This is an overflow observation, not a valid rate.
    let max_finite_base = f32::MAX / (60.0 * 5.0);
    if !max_finite_base.is_finite() {
        return Err("the derived finite-base boundary is not finite".to_string());
    }
    let overflow_config = FeedrateConfig {
        outer_wall_speed: max_finite_base * 0.5,
        ..Default::default()
    };
    let emitter =
        DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), overflow_config);
    let below = emitter
        .resolve_feedrate(&ExtrusionRole::OuterWall, 5.0)
        .ok_or_else(|| "overflow probe below the boundary returned None".to_string())?;
    if !below.is_finite() {
        return Err(format!(
            "base {} below the derived boundary must stay finite, got {below:?}",
            max_finite_base * 0.5
        ));
    }
    for beyond in [max_finite_base * 2.0, f32::MAX] {
        if !beyond.is_finite() {
            return Err(format!("overflow witness {beyond} is not a finite base"));
        }
        let config = FeedrateConfig {
            outer_wall_speed: beyond,
            ..Default::default()
        };
        let emitter = DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), config);
        match emitter.resolve_feedrate(&ExtrusionRole::OuterWall, 5.0) {
            Some(value) if value == f32::INFINITY => {}
            other => {
                return Err(format!(
                    "finite base {beyond} above {} must narrow to +Infinity, got {other:?}",
                    max_finite_base
                ))
            }
        }
    }

    // 5. Non-finite factors are boundary observations, never valid input:
    //    infinities saturate through f32::clamp, NaN propagates.
    let config = witness_feedrate_config(RoleSlot::OuterWall, 60.0);
    let emitter = DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), config);
    let positive = emitter
        .resolve_feedrate(&ExtrusionRole::OuterWall, f32::INFINITY)
        .ok_or_else(|| "+Infinity factor returned None".to_string())?;
    let at_high = emitter
        .resolve_feedrate(&ExtrusionRole::OuterWall, 5.0)
        .ok_or_else(|| "factor 5.0 returned None".to_string())?;
    if positive.to_bits() != at_high.to_bits() {
        return Err(format!(
            "boundary observation: +Infinity factor must saturate to the 5.0 clamp, got {positive:?} vs {at_high:?}"
        ));
    }
    let negative = emitter
        .resolve_feedrate(&ExtrusionRole::OuterWall, f32::NEG_INFINITY)
        .ok_or_else(|| "-Infinity factor returned None".to_string())?;
    let at_low = emitter
        .resolve_feedrate(&ExtrusionRole::OuterWall, 0.05)
        .ok_or_else(|| "factor 0.05 returned None".to_string())?;
    if negative.to_bits() != at_low.to_bits() {
        return Err(format!(
            "boundary observation: -Infinity factor must saturate to the 0.05 clamp, got {negative:?} vs {at_low:?}"
        ));
    }
    match emitter.resolve_feedrate(&ExtrusionRole::OuterWall, f32::NAN) {
        Some(value) if value.is_nan() => {}
        other => {
            return Err(format!(
                "boundary observation: a NaN factor propagates as NaN, got {other:?}"
            ))
        }
    }
    Ok(())
}

/// Build a feedrate config whose selected field for `slot` is exactly `base`
/// and whose every other field is a distinct constant.
fn witness_feedrate_config(slot: RoleSlot, base: f32) -> FeedrateConfig {
    let mut config = FeedrateConfig {
        outer_wall_speed: 31.0,
        inner_wall_speed: 32.0,
        thin_wall_speed: 33.0,
        top_surface_speed: 34.0,
        bottom_surface_speed: 35.0,
        sparse_infill_speed: 36.0,
        bridge_speed: 37.0,
        internal_bridge_speed: slicer_ir::resolved_config::ResolvedFloatOrPercent {
            value: 38.0,
            is_percent: false,
        },
        support_speed: 39.0,
        support_interface_speed: 40.0,
        gap_infill_speed: 41.0,
        ironing_speed: 42.0,
        skirt_speed: 43.0,
        wipe_tower_max_purge_speed: 44.0,
        prime_tower_speed: 45.0,
        travel_speed: 46.0,
        wipe_speed: 47.0,
        filament_ironing_speed: 0.0,
        ..Default::default()
    };
    match slot {
        RoleSlot::OuterWall => config.outer_wall_speed = base,
        RoleSlot::InnerWall => config.inner_wall_speed = base,
        RoleSlot::ThinWall => config.thin_wall_speed = base,
        RoleSlot::TopSolidInfill => config.top_surface_speed = base,
        RoleSlot::BottomSolidInfill => config.bottom_surface_speed = base,
        RoleSlot::InternalSolidInfill | RoleSlot::SparseInfill => {
            config.sparse_infill_speed = base;
        }
        RoleSlot::RaftInfill => config.outer_wall_speed = base,
        RoleSlot::BridgeInfill => config.bridge_speed = base,
        RoleSlot::InternalBridgeInfill => {
            config.internal_bridge_speed = slicer_ir::resolved_config::ResolvedFloatOrPercent {
                value: f64::from(base),
                is_percent: false,
            };
        }
        RoleSlot::SupportMaterial => config.support_speed = base,
        RoleSlot::SupportInterface | RoleSlot::SupportBaseInterface => {
            config.support_interface_speed = base;
        }
        RoleSlot::Skirt | RoleSlot::Brim => config.skirt_speed = base,
        RoleSlot::WipeTower => {
            // The merged emitter caps the wipe-tower rate by the sparse-infill
            // speed; pin both fields so the witness still selects `base`.
            config.wipe_tower_max_purge_speed = base;
            config.sparse_infill_speed = base;
        }
        RoleSlot::PrimeTower => config.prime_tower_speed = base,
        RoleSlot::GapFill => config.gap_infill_speed = base,
        RoleSlot::Ironing => {
            config.filament_ironing_speed = 0.0;
            config.ironing_speed = base;
        }
        RoleSlot::CustomTravel => config.travel_speed = base,
        RoleSlot::CustomWipe => config.wipe_speed = base,
        RoleSlot::CustomGapInfill => config.gap_infill_speed = base,
        RoleSlot::CustomOther => config.outer_wall_speed = base,
    }
    config
}

#[test]
fn proptest_feedrate_resolution() {
    let run = PropertyRun::new(FEEDRATE_PROPERTY_ID, FEEDRATE_TEST_NAME)
        .expect("H-GCODE-FEEDRATE registry context");
    let bounds = feedrate_bounds(&run).expect("H-GCODE-FEEDRATE checked profile bounds");
    feedrate_deterministic_witnesses().expect("H-GCODE-FEEDRATE deterministic witnesses");
    let strategy = feedrate_case_strategy(bounds);
    run.check(&strategy, |case, _guard| {
        check_feedrate_case(&case, FeedrateMutation::None).map_err(TestCaseError::fail)
    })
    .expect("H-GCODE-FEEDRATE role table, clamp, conversion and round-to-millidegree");
}

#[test]
fn gcode_feedrate_controls() {
    // The table fixture and the independent table agree on every named role.
    for slot in RoleSlot::ALL {
        for base in [0.0_f32, 7.5, 60.0, 1000.0] {
            let config = witness_feedrate_config(slot, base);
            let selected = expected_base_speed_mm_per_s(&config, slot);
            assert_eq!(
                selected.to_bits(),
                base.to_bits(),
                "the witness fixture for {slot:?} does not select the pinned base {base}"
            );
            check_resolved_feedrate(&config, slot, 1.0, FeedrateMutation::None)
                .unwrap_or_else(|error| panic!("documented table rejected {slot:?}: {error}"));
        }
    }

    // Internal-bridge percent-vs-absolute rule, both directions.
    let mut percent = FeedrateConfig {
        bridge_speed: 20.0,
        internal_bridge_speed: slicer_ir::resolved_config::ResolvedFloatOrPercent {
            value: 50.0,
            is_percent: true,
        },
        ..Default::default()
    };
    assert_eq!(
        expected_base_speed_mm_per_s(&percent, RoleSlot::InternalBridgeInfill),
        10.0
    );
    percent.internal_bridge_speed = slicer_ir::resolved_config::ResolvedFloatOrPercent {
        value: 12.0,
        is_percent: false,
    };
    assert_eq!(
        expected_base_speed_mm_per_s(&percent, RoleSlot::InternalBridgeInfill),
        12.0
    );

    // Ironing: the filament override wins only when strictly positive.
    let mut ironing = FeedrateConfig {
        ironing_speed: 20.0,
        filament_ironing_speed: 0.0,
        ..Default::default()
    };
    assert_eq!(
        expected_base_speed_mm_per_s(&ironing, RoleSlot::Ironing),
        20.0
    );
    ironing.filament_ironing_speed = 15.0;
    assert_eq!(
        expected_base_speed_mm_per_s(&ironing, RoleSlot::Ironing),
        15.0
    );

    // Every mutation must be rejected by at least one witness, and the
    // documented table must accept that same witness (so the rejection is the
    // mutation, not a broken fixture).
    let witnesses = [
        FeedrateCase {
            slot: RoleSlot::OuterWall,
            base_milli: 60_000,
            factor_mode: FactorMode::Inside,
            factor_magnitude: 1_000,
            bridge_percent: false,
            ironing_filament_override: false,
        },
        FeedrateCase {
            slot: RoleSlot::Skirt,
            base_milli: 60_000,
            factor_mode: FactorMode::BelowLow,
            factor_magnitude: 10,
            bridge_percent: false,
            ironing_filament_override: false,
        },
        FeedrateCase {
            slot: RoleSlot::WipeTower,
            base_milli: 60_000,
            factor_mode: FactorMode::AboveHigh,
            factor_magnitude: 5_500,
            bridge_percent: false,
            ironing_filament_override: false,
        },
        FeedrateCase {
            slot: RoleSlot::SparseInfill,
            base_milli: 25,
            factor_mode: FactorMode::Inside,
            factor_magnitude: 1_667,
            bridge_percent: false,
            ironing_filament_override: false,
        },
        FeedrateCase {
            slot: RoleSlot::Ironing,
            base_milli: 0,
            factor_mode: FactorMode::Zero,
            factor_magnitude: 0,
            bridge_percent: false,
            ironing_filament_override: false,
        },
        FeedrateCase {
            slot: RoleSlot::InternalBridgeInfill,
            base_milli: 250_000,
            factor_mode: FactorMode::ExactHigh,
            factor_magnitude: 0,
            bridge_percent: true,
            ironing_filament_override: false,
        },
    ];
    let mutations = [
        FeedrateMutation::MissingConversion,
        FeedrateMutation::UnitClamp,
        FeedrateMutation::TruncateMillidegree,
        FeedrateMutation::RoleSwap,
        FeedrateMutation::ExactRoundtrip,
    ];
    for mutation in mutations {
        let mut rejected = 0_usize;
        for case in &witnesses {
            let config = feedrate_config_for_case(case);
            check_resolved_feedrate(&config, case.slot, case.factor(), FeedrateMutation::None)
                .unwrap_or_else(|error| {
                    panic!("documented table rejected the control witness {case:?}: {error}")
                });
            let documented =
                expected_feedrate(&config, case.slot, case.factor(), FeedrateMutation::None);
            let mutant = expected_feedrate(&config, case.slot, case.factor(), mutation);
            if documented.to_bits() == mutant.to_bits() {
                continue;
            }
            assert!(
                check_resolved_feedrate(&config, case.slot, case.factor(), mutation).is_err(),
                "mutation {mutation:?} was not rejected on witness {case:?} (documented {documented:?}, mutant {mutant:?})"
            );
            rejected += 1;
        }
        assert!(
            rejected > 0,
            "mutation {mutation:?} produced no distinguishable witness; the control is vacuous"
        );
    }

    // The role table is exhaustive: every real IR role maps to a slot whose
    // documented table entry is selected by the witness fixture.
    for role in [
        ExtrusionRole::OuterWall,
        ExtrusionRole::InnerWall,
        ExtrusionRole::ThinWall,
        ExtrusionRole::TopSolidInfill,
        ExtrusionRole::BottomSolidInfill,
        ExtrusionRole::InternalSolidInfill,
        ExtrusionRole::SparseInfill,
        ExtrusionRole::RaftInfill,
        ExtrusionRole::SupportMaterial,
        ExtrusionRole::SupportInterface,
        ExtrusionRole::SupportBaseInterface,
        ExtrusionRole::WipeTower,
        ExtrusionRole::PrimeTower,
        ExtrusionRole::Ironing,
        ExtrusionRole::BridgeInfill,
        ExtrusionRole::InternalBridgeInfill,
        ExtrusionRole::Skirt,
        ExtrusionRole::Brim,
        ExtrusionRole::GapFill,
        ExtrusionRole::Custom("Travel".to_string()),
        ExtrusionRole::Custom("Wipe".to_string()),
        ExtrusionRole::Custom("GapInfill".to_string()),
        ExtrusionRole::Custom("property-other".to_string()),
    ] {
        let slot = RoleSlot::from_role(&role);
        let config = witness_feedrate_config(slot, 55.0);
        check_resolved_feedrate(&config, slot, 2.0, FeedrateMutation::None)
            .unwrap_or_else(|error| panic!("role {role:?} failed its own table: {error}"));
    }

    // Non-vacuous shrink/complicate probe over the same strategy and checker.
    let strategy = feedrate_case_strategy(CONTROL_FEEDRATE_BOUNDS);
    shrink_complicate_witness(
        FEEDRATE_PROPERTY_ID,
        &strategy,
        |case| Some(format!("{:?}/{:?}", case.slot, case.factor_mode)),
        |case| check_feedrate_case(case, FeedrateMutation::None),
    );
}

// ════════════════════════════════════════════════════════════════════════════
// H-GCODE-EMIT: a small command/state model over bounded scenes
// ════════════════════════════════════════════════════════════════════════════

/// Registry identity of the emission charter.
const EMIT_PROPERTY_ID: &str = "H-GCODE-EMIT";
/// Exact registry test name of the emission charter.
const EMIT_TEST_NAME: &str = "proptest_emission_state_sequence";

/// Roles exercised by the positive emission domain.
///
/// All are non-loop roles, so the emitter's unclosed-loop warning cannot fire;
/// every one has a nonzero configured base speed in the fixture, so `f` always
/// follows the explicit feedrate path and the automatic-volumetric branch is
/// reserved for the labelled negative modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitRole {
    SparseInfill,
    TopSolidInfill,
    InternalSolidInfill,
    GapFill,
    Ironing,
    BridgeInfill,
    SupportMaterial,
}

impl EmitRole {
    const ALL: [EmitRole; 7] = [
        EmitRole::SparseInfill,
        EmitRole::TopSolidInfill,
        EmitRole::InternalSolidInfill,
        EmitRole::GapFill,
        EmitRole::Ironing,
        EmitRole::BridgeInfill,
        EmitRole::SupportMaterial,
    ];

    fn role(self) -> ExtrusionRole {
        match self {
            EmitRole::SparseInfill => ExtrusionRole::SparseInfill,
            EmitRole::TopSolidInfill => ExtrusionRole::TopSolidInfill,
            EmitRole::InternalSolidInfill => ExtrusionRole::InternalSolidInfill,
            EmitRole::GapFill => ExtrusionRole::GapFill,
            EmitRole::Ironing => ExtrusionRole::Ironing,
            EmitRole::BridgeInfill => ExtrusionRole::BridgeInfill,
            EmitRole::SupportMaterial => ExtrusionRole::SupportMaterial,
        }
    }
}

/// One generated point: `1/8` mm dyadic XY units plus an exact-duplicate flag
/// that forces a zero-length segment without relying on RNG coincidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EmitPointSpec {
    x_units: i32,
    y_units: i32,
    repeat_previous: bool,
}

/// One generated entity slot.
#[derive(Debug, Clone, PartialEq)]
struct EmitRawEntity {
    role: EmitRole,
    speed_factor_milli: u32,
    points: Vec<EmitPointSpec>,
}

/// Annotation kind for the generated side table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitAnnotationFlavor {
    Comment,
    Raw,
}

/// One generated annotation; `trailing` anchors it past the last entity.
#[derive(Debug, Clone, PartialEq)]
struct EmitRawAnnotation {
    trailing: bool,
    flavor: EmitAnnotationFlavor,
    text: String,
}

/// One generated retract/unretract decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EmitRawRetract {
    length_units: u32,
    speed_units: u32,
    is_unretract: bool,
    firmware: bool,
}

/// One generated travel destination (`None` feed override: the emitter's own
/// travel feedrate is used, so the oracle stays exact).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EmitRawTravel {
    x_units: i32,
    y_units: i32,
}

/// One generated tool change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EmitRawToolChange {
    from: u32,
    to: u32,
}

/// One generated layer.
#[derive(Debug, Clone, PartialEq)]
struct EmitRawLayer {
    /// Exactly `max_entities_per_layer` slots; `entity_count` truncates them.
    entity_slots: Vec<EmitRawEntity>,
    entity_count: usize,
    tool_changes: Vec<EmitRawToolChange>,
    z_hop_units: Option<u32>,
    retracts: Vec<EmitRawRetract>,
    travels: Vec<EmitRawTravel>,
    annotations: Vec<EmitRawAnnotation>,
    /// Optional per-point factor row for the layer's first entity.
    profile: Option<Vec<u32>>,
}

/// A bounded scene. The value tree shrinks layers, entities, points and side
/// entries; the per-layer z and the #11 chain are structural constants.
#[derive(Debug, Clone, PartialEq)]
struct EmitRawScene {
    layers: Vec<EmitRawLayer>,
}

/// Checked registry bounds for the emission charter.
#[derive(Debug, Clone, Copy)]
struct EmitBounds {
    max_layers: usize,
    max_entities: usize,
    max_points: usize,
    max_abs_mm: i32,
}

impl EmitBounds {
    fn coordinate_limit(self) -> i32 {
        self.max_abs_mm * 8
    }
}

/// Test-local profile mirroring the registry's measured routine bounds.
const CONTROL_EMIT_BOUNDS: EmitBounds = EmitBounds {
    max_layers: 3,
    max_entities: 3,
    max_points: 4,
    max_abs_mm: 64,
};

fn emit_bounds(run: &PropertyRun) -> Result<EmitBounds, String> {
    let max_layers = usize::try_from(positive_bound(run, "max_layers")?)
        .map_err(|error| format!("bound max_layers: {error}"))?;
    let max_entities = usize::try_from(positive_bound(run, "max_entities_per_layer")?)
        .map_err(|error| format!("bound max_entities_per_layer: {error}"))?;
    let max_points = usize::try_from(positive_bound(run, "max_points_per_entity")?)
        .map_err(|error| format!("bound max_points_per_entity: {error}"))?;
    let max_abs_mm = i32::try_from(positive_bound(run, "max_abs_mm")?)
        .map_err(|error| format!("bound max_abs_mm: {error}"))?;
    let bounds = EmitBounds {
        max_layers,
        max_entities,
        max_points,
        max_abs_mm,
    };
    let limit = bounds.max_abs_mm.checked_mul(8).ok_or_else(|| {
        format!(
            "bound max_abs_mm {} overflows 1/8 mm units",
            bounds.max_abs_mm
        )
    })?;
    if limit <= 0 {
        return Err("bound max_abs_mm must yield a positive coordinate limit".to_string());
    }
    if bounds.max_layers > 8 || bounds.max_entities > 8 || bounds.max_points > 16 {
        return Err(format!(
            "emission profile {bounds:?} exceeds the declared allocation guards (8 layers, 8 entities, 16 points)"
        ));
    }
    Ok(bounds)
}

fn property_ascii_text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::char::range('a', 'z'), 0..=6)
        .prop_map(|characters| characters.into_iter().collect())
}

fn emit_point_strategy(bounds: EmitBounds) -> impl Strategy<Value = EmitPointSpec> {
    let limit = bounds.coordinate_limit();
    (
        -limit..=limit,
        -limit..=limit,
        select(vec![false, true]).no_shrink(),
    )
        .prop_map(|(x_units, y_units, repeat_previous)| EmitPointSpec {
            x_units,
            y_units,
            repeat_previous,
        })
}

fn emit_entity_strategy(bounds: EmitBounds) -> impl Strategy<Value = EmitRawEntity> {
    (
        select(EmitRole::ALL.to_vec()).no_shrink(),
        0..=10_000_u32,
        0..=bounds.max_points,
        prop::collection::vec(emit_point_strategy(bounds), bounds.max_points),
    )
        .prop_map(
            |(role, speed_factor_milli, point_count, slots)| EmitRawEntity {
                role,
                speed_factor_milli,
                points: slots.into_iter().take(point_count).collect(),
            },
        )
}

fn emit_layer_strategy(bounds: EmitBounds) -> impl Strategy<Value = EmitRawLayer> {
    let limit = bounds.coordinate_limit();
    let tool_change = (0..=2_u32, 0..=2_u32).prop_map(|(from, to)| EmitRawToolChange { from, to });
    let retract = (
        0..=64_u32,
        0..=4_800_u32,
        select(vec![false, true]).no_shrink(),
        select(vec![false, true]).no_shrink(),
    )
        .prop_map(
            |(length_units, speed_units, is_unretract, firmware)| EmitRawRetract {
                length_units,
                speed_units,
                is_unretract,
                firmware,
            },
        );
    let travel = (-limit..=limit, -limit..=limit)
        .prop_map(|(x_units, y_units)| EmitRawTravel { x_units, y_units });
    let annotation = (
        select(vec![false, true]).no_shrink(),
        select(vec![false, true]).no_shrink(),
        property_ascii_text(),
    )
        .prop_map(|(trailing, raw, text)| EmitRawAnnotation {
            trailing,
            flavor: if raw {
                EmitAnnotationFlavor::Raw
            } else {
                EmitAnnotationFlavor::Comment
            },
            text,
        });
    (
        prop::collection::vec(emit_entity_strategy(bounds), bounds.max_entities),
        0..=bounds.max_entities,
        prop::collection::vec(tool_change, 0..=1),
        prop::option::of(1..=8_u32),
        prop::collection::vec(retract, 0..=2),
        prop::collection::vec(travel, 0..=2),
        prop::collection::vec(annotation, 0..=2),
        prop::option::of(prop::collection::vec(0..=10_000_u32, 0..=bounds.max_points)),
    )
        .prop_map(
            |(
                entity_slots,
                entity_count,
                tool_changes,
                z_hop_units,
                retracts,
                travels,
                annotations,
                profile,
            )| EmitRawLayer {
                entity_slots,
                entity_count,
                tool_changes,
                z_hop_units,
                retracts,
                travels,
                annotations,
                profile,
            },
        )
}

fn emit_scene_strategy(bounds: EmitBounds) -> impl Strategy<Value = EmitRawScene> {
    prop::collection::vec(emit_layer_strategy(bounds), 0..=bounds.max_layers)
        .prop_map(|layers| EmitRawScene { layers })
}

/// The #11-approved ordered chain: `material` (priority 100) precedes
/// `fuzzy_skin` (priority 200). Supplied as fixture input only; `GCodeIR` has no
/// `RegionKey`, so output preservation is never claimed.
fn property_variant_chain() -> Vec<(String, slicer_ir::PaintValue)> {
    vec![
        ("material".to_string(), slicer_ir::PaintValue::ToolIndex(1)),
        ("fuzzy_skin".to_string(), slicer_ir::PaintValue::Flag(true)),
    ]
}

fn emit_point(spec: &EmitPointSpec, z: f32) -> slicer_ir::Point3WithWidth {
    slicer_ir::Point3WithWidth {
        x: spec.x_units as f32 / 8.0,
        y: spec.y_units as f32 / 8.0,
        z,
        width: 0.5,
        flow_factor: 1.0,
        ..Default::default()
    }
}

/// Materialize a generated scene. Entity ids are assigned from one monotonic
/// counter, so they are distinct within every layer. `order_lock = Some(0)`
/// bypasses both simplification and short-segment pruning, which keeps the
/// oracle's point set identical to the input point set.
fn build_emission_layers(scene: &EmitRawScene) -> Vec<slicer_ir::LayerCollectionIR> {
    let mut next_entity_id = 1_u64;
    let mut layers = Vec::with_capacity(scene.layers.len());
    for (layer_index, raw_layer) in scene.layers.iter().enumerate() {
        let z = 0.25 * (layer_index as f32 + 1.0);
        let count = raw_layer.entity_count.min(raw_layer.entity_slots.len());
        let mut entities = Vec::with_capacity(count);
        for slot in raw_layer.entity_slots.iter().take(count) {
            let entity_id = next_entity_id;
            next_entity_id += 1;
            let mut points = Vec::with_capacity(slot.points.len());
            let mut previous: Option<slicer_ir::Point3WithWidth> = None;
            for spec in &slot.points {
                let point = if spec.repeat_previous {
                    previous.unwrap_or_else(|| emit_point(spec, z))
                } else {
                    emit_point(spec, z)
                };
                previous = Some(point);
                points.push(point);
            }
            let role = slot.role.role();
            entities.push(slicer_ir::PrintEntity {
                entity_id,
                path: slicer_ir::ExtrusionPath3D {
                    points,
                    role: role.clone(),
                    speed_factor: slot.speed_factor_milli as f32 / 1000.0,
                    order_lock: Some(0),
                    ..slicer_sdk::test_support::fixtures::extrusion_path3d_base(role.clone())
                },
                role: role.clone(),
                region_key: slicer_ir::RegionKey {
                    global_layer_index: layer_index as u32,
                    object_id: "property-gcode-object".to_string(),
                    region_id: 0,
                    variant_chain: property_variant_chain(),
                },
                tool_index: 0,
                ..slicer_sdk::test_support::fixtures::print_entity_base(role)
            });
        }

        let anchor = |index: usize| -> u32 {
            if count == 0 {
                index as u32
            } else {
                (index as u32).min(count as u32 - 1)
            }
        };
        let tool_changes = raw_layer
            .tool_changes
            .iter()
            .enumerate()
            .map(|(index, tool_change)| slicer_ir::ToolChange {
                after_entity_index: anchor(index),
                from_tool: tool_change.from,
                to_tool: tool_change.to,
            })
            .collect();
        let z_hops = raw_layer
            .z_hop_units
            .map(|units| slicer_ir::ZHop {
                after_entity_index: count.saturating_sub(1) as u32,
                hop_height: units as f32 / 4.0,
            })
            .into_iter()
            .collect();
        let retracts = raw_layer
            .retracts
            .iter()
            .enumerate()
            .map(|(index, retract)| {
                // exhaustive: all fields are generated from the case.
                slicer_ir::TravelRetract {
                    after_entity_index: anchor(index),
                    length: retract.length_units as f32 / 32.0,
                    speed: retract.speed_units as f32 / 16.0,
                    is_unretract: retract.is_unretract,
                    mode: if retract.firmware {
                        slicer_ir::RetractMode::Firmware
                    } else {
                        slicer_ir::RetractMode::Gcode
                    },
                }
            })
            .collect();
        let travels = raw_layer
            .travels
            .iter()
            .enumerate()
            .map(|(index, travel)| slicer_ir::TravelMove {
                entity_id: if count == 0 {
                    u64::MAX
                } else {
                    entities[index % count].entity_id
                },
                x: Some(travel.x_units as f32 / 8.0),
                y: Some(travel.y_units as f32 / 8.0),
                f: None,
                ..slicer_ir::TravelMove::default()
            })
            .collect();
        let annotations = raw_layer
            .annotations
            .iter()
            .enumerate()
            .map(|(index, annotation)| slicer_ir::LayerAnnotation {
                after_entity_index: if annotation.trailing {
                    count as u32
                } else {
                    anchor(index)
                },
                kind: match annotation.flavor {
                    EmitAnnotationFlavor::Comment => {
                        slicer_ir::LayerAnnotationKind::Comment(annotation.text.clone())
                    }
                    EmitAnnotationFlavor::Raw => {
                        slicer_ir::LayerAnnotationKind::Raw(annotation.text.clone())
                    }
                },
            })
            .collect();
        let speed_profiles = match (&raw_layer.profile, count) {
            (Some(factors), count) if count > 0 => vec![slicer_ir::EntitySpeedProfile {
                entity_id: entities[0].entity_id,
                factors: factors.iter().map(|milli| *milli as f32 / 1000.0).collect(),
            }],
            _ => Vec::new(),
        };
        layers.push(slicer_ir::LayerCollectionIR {
            global_layer_index: layer_index as u32,
            z,
            ordered_entities: entities,
            tool_changes,
            z_hops,
            annotations,
            retracts,
            travel_moves: travels,
            speed_profiles,
            ..Default::default()
        });
    }
    layers
}

/// Feedrate configuration for the emission charter: every role base speed is a
/// distinct nonzero value, so a role-table mix-up changes `f`.
fn emission_feedrate_config() -> FeedrateConfig {
    FeedrateConfig {
        outer_wall_speed: 40.0,
        inner_wall_speed: 45.0,
        thin_wall_speed: 50.0,
        top_surface_speed: 55.0,
        bottom_surface_speed: 60.0,
        sparse_infill_speed: 65.0,
        bridge_speed: 30.0,
        internal_bridge_speed: slicer_ir::resolved_config::ResolvedFloatOrPercent {
            value: 35.0,
            is_percent: false,
        },
        support_speed: 70.0,
        support_interface_speed: 75.0,
        gap_infill_speed: 80.0,
        ironing_speed: 20.0,
        skirt_speed: 25.0,
        wipe_tower_max_purge_speed: 85.0,
        prime_tower_speed: 90.0,
        travel_speed: 120.0,
        wipe_speed: 95.0,
        filament_ironing_speed: 0.0,
        ..Default::default()
    }
}

/// Resolved configuration for the emission charter: M73 disabled (the header
/// block is not modelled here), wipe-tower retract synthesis disabled in the
/// positive domain, and a dyadic first-layer height.
fn emission_resolved_config() -> slicer_ir::ResolvedConfig {
    slicer_ir::ResolvedConfig {
        disable_m73: true,
        enable_prime_tower: false,
        filament_diameter: 1.75,
        initial_layer_print_height: 0.25,
        retract_length: 2.0,
        filament_density: Vec::new(),
        ..slicer_ir::ResolvedConfig::default()
    }
}

/// The documented `;TYPE:` label for a role.
fn model_type_label(role: &ExtrusionRole) -> &'static str {
    match role {
        ExtrusionRole::OuterWall => ";TYPE:Outer wall",
        ExtrusionRole::InnerWall => ";TYPE:Inner wall",
        ExtrusionRole::ThinWall => ";TYPE:Inner wall",
        ExtrusionRole::TopSolidInfill => ";TYPE:Top surface",
        ExtrusionRole::BottomSolidInfill => ";TYPE:Bottom surface",
        ExtrusionRole::InternalSolidInfill => ";TYPE:Internal solid infill",
        ExtrusionRole::SparseInfill => ";TYPE:Sparse infill",
        ExtrusionRole::BridgeInfill => ";TYPE:Bridge",
        ExtrusionRole::InternalBridgeInfill => ";TYPE:Internal Bridge",
        ExtrusionRole::SupportMaterial => ";TYPE:Support",
        ExtrusionRole::SupportInterface => ";TYPE:Support interface",
        ExtrusionRole::SupportBaseInterface => ";TYPE:Support interface",
        ExtrusionRole::Skirt => ";TYPE:Skirt",
        ExtrusionRole::Brim => ";TYPE:Brim",
        ExtrusionRole::WipeTower => ";TYPE:Prime tower",
        ExtrusionRole::PrimeTower => ";TYPE:Prime tower",
        ExtrusionRole::Ironing => ";TYPE:Ironing",
        ExtrusionRole::Custom(_) => ";TYPE:Custom",
        ExtrusionRole::GapFill => ";TYPE:Gap infill",
        ExtrusionRole::RaftInfill => ";TYPE:Support",
    }
}

/// One predicted command. `Move.has_e` replaces the exact E value: the
/// property asserts E presence, strict cumulative growth and per-tool
/// bookkeeping rather than cloning the emitter's volumetric geometry.
#[derive(Debug, Clone, PartialEq)]
enum ModelCommand {
    Move {
        x: Option<f32>,
        y: Option<f32>,
        z: Option<f32>,
        has_e: bool,
        f: Option<f32>,
        role: ExtrusionRole,
    },
    Retract {
        length: f32,
        speed: f32,
        mode: slicer_ir::RetractMode,
    },
    Unretract {
        length: f32,
        speed: f32,
        mode: slicer_ir::RetractMode,
    },
    ToolChange {
        after_entity_index: u32,
        from: u32,
        to: u32,
    },
    Comment {
        text: String,
    },
    Raw {
        text: String,
    },
    ExtrusionMode {
        absolute: bool,
    },
}

/// Predicted stream plus the metadata facts the oracle derives from the input.
#[derive(Debug, Clone, PartialEq)]
struct EmissionModel {
    commands: Vec<ModelCommand>,
    layer_count: u32,
    tools_with_moves: std::collections::BTreeSet<u32>,
    max_tool: u32,
}

/// The independent emission state machine from the charter.
fn build_emission_model(
    layers: &[slicer_ir::LayerCollectionIR],
    feedrate: &FeedrateConfig,
    resolved: &slicer_ir::ResolvedConfig,
) -> EmissionModel {
    let mut commands = vec![ModelCommand::ExtrusionMode { absolute: false }];
    let mut emitted_layers = 0_u32;
    let mut current_tool = 0_u32;
    let mut prev_layer_z: Option<f32> = None;
    let initial_layer_print_height = resolved.initial_layer_print_height as f32;
    let mut prev_role: Option<ExtrusionRole> = None;
    let mut tools_with_moves = std::collections::BTreeSet::new();
    let travel_f = feedrate_from_base_mm_per_s(
        expected_base_speed_mm_per_s(feedrate, RoleSlot::CustomTravel),
        1.0,
    );
    let travel_role = ExtrusionRole::Custom("Travel".to_string());

    for layer in layers {
        let has_output = !layer.ordered_entities.is_empty()
            || !layer.travel_moves.is_empty()
            || !layer.retracts.is_empty()
            || !layer.tool_changes.is_empty()
            || !layer.z_hops.is_empty();
        if !has_output {
            continue;
        }
        emitted_layers += 1;

        let height_delta = match prev_layer_z {
            Some(previous) => layer.z - previous,
            None => initial_layer_print_height,
        };
        prev_layer_z = Some(layer.z);
        commands.push(ModelCommand::Raw {
            text: ";LAYER_CHANGE".to_string(),
        });
        commands.push(ModelCommand::Raw {
            text: format!(";Z:{}", oracle_coord(layer.z, 3)),
        });
        commands.push(ModelCommand::Raw {
            text: format!(";HEIGHT:{}", oracle_coord(height_delta, 3)),
        });

        if let Some(first) = layer.ordered_entities.first() {
            if first.tool_index != current_tool {
                commands.push(ModelCommand::ToolChange {
                    after_entity_index: u32::MAX,
                    from: current_tool,
                    to: first.tool_index,
                });
                current_tool = first.tool_index;
            }
        }

        let mut tool_changes: std::collections::BTreeMap<u32, &slicer_ir::ToolChange> =
            std::collections::BTreeMap::new();
        for tool_change in &layer.tool_changes {
            tool_changes.insert(tool_change.after_entity_index, tool_change);
        }
        let mut z_hops: std::collections::BTreeMap<u32, &slicer_ir::ZHop> =
            std::collections::BTreeMap::new();
        for z_hop in &layer.z_hops {
            z_hops.insert(z_hop.after_entity_index, z_hop);
        }
        let mut retracts_by_entity: std::collections::BTreeMap<
            u32,
            Vec<&slicer_ir::TravelRetract>,
        > = std::collections::BTreeMap::new();
        for retract in &layer.retracts {
            retracts_by_entity
                .entry(retract.after_entity_index)
                .or_default()
                .push(retract);
        }
        let mut travels_by_entity: std::collections::BTreeMap<u64, Vec<&slicer_ir::TravelMove>> =
            std::collections::BTreeMap::new();
        for travel in &layer.travel_moves {
            travels_by_entity
                .entry(travel.entity_id)
                .or_default()
                .push(travel);
        }
        let mut profiles_by_entity: std::collections::BTreeMap<u64, &Vec<f32>> =
            std::collections::BTreeMap::new();
        for profile in &layer.speed_profiles {
            profiles_by_entity.insert(profile.entity_id, &profile.factors);
        }
        let entity_count = layer.ordered_entities.len() as u32;
        let mut annotations_by_entity: std::collections::BTreeMap<
            u32,
            Vec<&slicer_ir::LayerAnnotation>,
        > = std::collections::BTreeMap::new();
        let mut trailing_annotations: Vec<&slicer_ir::LayerAnnotation> = Vec::new();
        for annotation in &layer.annotations {
            if annotation.after_entity_index < entity_count {
                annotations_by_entity
                    .entry(annotation.after_entity_index)
                    .or_default()
                    .push(annotation);
            } else {
                trailing_annotations.push(annotation);
            }
        }

        for (entity_index, entity) in layer.ordered_entities.iter().enumerate() {
            let entity_index = entity_index as u32;
            let role = &entity.path.role;
            let role_changed = prev_role.as_ref().is_none_or(|previous| previous != role);
            if role_changed {
                commands.push(ModelCommand::Raw {
                    text: model_type_label(role).to_string(),
                });
            }
            prev_role = Some(role.clone());

            let profile = profiles_by_entity.get(&entity.entity_id).copied();
            let mut previous: Option<&slicer_ir::Point3WithWidth> = None;
            for (point_index, point) in entity.path.points.iter().enumerate() {
                let zero_length = previous.is_some_and(|prior| {
                    prior.x == point.x && prior.y == point.y && prior.z == point.z
                });
                let has_e = previous.is_some() && !zero_length;
                let factor = profile
                    .and_then(|factors| factors.get(point_index).copied())
                    .unwrap_or(entity.path.speed_factor);
                let base = expected_base_speed_mm_per_s(feedrate, RoleSlot::from_role(role));
                commands.push(ModelCommand::Move {
                    x: Some(point.x),
                    y: Some(point.y),
                    z: Some(point.z),
                    has_e,
                    f: Some(feedrate_from_base_mm_per_s(base, factor)),
                    role: role.clone(),
                });
                tools_with_moves.insert(current_tool);
                previous = Some(point);
            }

            if let Some(list) = retracts_by_entity.get(&entity_index) {
                for retract in list.iter().filter(|retract| !retract.is_unretract) {
                    commands.push(ModelCommand::Retract {
                        length: retract.length,
                        speed: retract.speed,
                        mode: retract.mode,
                    });
                }
            }
            if let Some(z_hop) = z_hops.get(&entity_index) {
                commands.push(ModelCommand::Move {
                    x: None,
                    y: None,
                    z: Some(layer.z + z_hop.hop_height),
                    has_e: false,
                    f: Some(travel_f),
                    role: travel_role.clone(),
                });
            }
            if let Some(list) = travels_by_entity.get(&entity.entity_id) {
                for travel in list {
                    commands.push(ModelCommand::Move {
                        x: travel.x,
                        y: travel.y,
                        z: None,
                        has_e: false,
                        f: Some(travel_f),
                        role: travel_role.clone(),
                    });
                }
            }
            if let Some(tool_change) = tool_changes.get(&entity_index) {
                commands.push(ModelCommand::ToolChange {
                    after_entity_index: tool_change.after_entity_index,
                    from: tool_change.from_tool,
                    to: tool_change.to_tool,
                });
                current_tool = tool_change.to_tool;
                prev_role = None;
            }
            if let Some(list) = annotations_by_entity.get(&entity_index) {
                for annotation in list {
                    match &annotation.kind {
                        slicer_ir::LayerAnnotationKind::Comment(text) => {
                            commands.push(ModelCommand::Comment { text: text.clone() });
                        }
                        slicer_ir::LayerAnnotationKind::Raw(text) => {
                            commands.push(ModelCommand::Raw { text: text.clone() });
                        }
                    }
                }
            }
            if z_hops.contains_key(&entity_index) {
                commands.push(ModelCommand::Move {
                    x: None,
                    y: None,
                    z: Some(layer.z),
                    has_e: false,
                    f: Some(travel_f),
                    role: travel_role.clone(),
                });
            }
            if let Some(list) = retracts_by_entity.get(&entity_index) {
                for retract in list.iter().filter(|retract| retract.is_unretract) {
                    commands.push(ModelCommand::Unretract {
                        length: retract.length,
                        speed: retract.speed,
                        mode: retract.mode,
                    });
                }
            }
        }

        for annotation in trailing_annotations {
            match &annotation.kind {
                slicer_ir::LayerAnnotationKind::Comment(text) => {
                    commands.push(ModelCommand::Comment { text: text.clone() });
                }
                slicer_ir::LayerAnnotationKind::Raw(text) => {
                    commands.push(ModelCommand::Raw { text: text.clone() });
                }
            }
        }
    }

    let max_tool = tools_with_moves.iter().max().copied().unwrap_or(0);
    EmissionModel {
        commands,
        layer_count: emitted_layers,
        tools_with_moves,
        max_tool,
    }
}

/// Whether a raw line belongs to the emitter's trailing filament statistics.
fn is_filament_stats_line(text: &str) -> bool {
    text.starts_with("; filament used [mm] = ")
        || text.starts_with("; filament used [cm3] = ")
        || text.starts_with("; filament used [g] = ")
        || text.starts_with("; estimated printing time (normal mode) = ")
}

/// Split the documented trailing filament-statistics comment block from the
/// layer walk, validating its documented shape.
fn split_filament_stats_tail(
    commands: &[GCodeCommand],
) -> Result<(&[GCodeCommand], &[GCodeCommand]), String> {
    let mut start = commands.len();
    while start > 0 {
        match &commands[start - 1] {
            GCodeCommand::Raw { text } if is_filament_stats_line(text) => start -= 1,
            _ => break,
        }
    }
    let tail = &commands[start..];
    if tail.is_empty() {
        return Err(
            "the emitter always appends a filament-statistics comment block; none was found"
                .to_string(),
        );
    }
    let line_matches = |index: usize, prefix: &str| matches!(&tail[index], GCodeCommand::Raw { text } if text.starts_with(prefix));
    if !line_matches(0, "; filament used [mm] = ") {
        return Err(format!(
            "the filament-statistics block must start with `; filament used [mm] = `, got {:?}",
            tail[0]
        ));
    }
    if tail.len() < 3 {
        return Err(format!(
            "the filament-statistics block must carry at least mm/cm3/time lines, got {tail:?}"
        ));
    }
    if !line_matches(1, "; filament used [cm3] = ") {
        return Err(format!(
            "the second filament-statistics line must be `; filament used [cm3] = `, got {:?}",
            tail[1]
        ));
    }
    let last = tail.len() - 1;
    if !line_matches(last, "; estimated printing time (normal mode) = ") {
        return Err(format!(
            "the filament-statistics block must end with the estimated-printing-time line, got {:?}",
            tail[last]
        ));
    }
    match tail.len() {
        3 => {}
        4 if line_matches(2, "; filament used [g] = ") => {}
        other => {
            return Err(format!(
            "the filament-statistics block has an undocumented shape with {other} lines: {tail:?}"
        ))
        }
    }
    Ok((&commands[..start], tail))
}

/// Compare the real command stream and metadata against the emission model.
fn compare_emission_stream(model: &EmissionModel, ir: &GCodeIR) -> Result<(), String> {
    let (walk, _tail) = split_filament_stats_tail(&ir.commands)?;
    if walk.len() != model.commands.len() {
        return Err(format!(
            "the emitter produced {} layer-walk commands, the state model predicts {}",
            walk.len(),
            model.commands.len()
        ));
    }
    let mut last_e: Option<f32> = None;
    for (index, (observed, expected)) in walk.iter().zip(model.commands.iter()).enumerate() {
        match (observed, expected) {
            (
                GCodeCommand::Move {
                    x,
                    y,
                    z,
                    e,
                    f,
                    role,
                },
                ModelCommand::Move {
                    x: expected_x,
                    y: expected_y,
                    z: expected_z,
                    has_e,
                    f: expected_f,
                    role: expected_role,
                },
            ) => {
                if x != expected_x || y != expected_y || z != expected_z {
                    return Err(format!(
                        "command {index}: Move coordinates ({x:?}, {y:?}, {z:?}) do not match the input points ({expected_x:?}, {expected_y:?}, {expected_z:?})"
                    ));
                }
                if f != expected_f {
                    return Err(format!(
                        "command {index}: Move f {f:?} does not match the feedrate oracle {expected_f:?}"
                    ));
                }
                if role != expected_role {
                    return Err(format!(
                        "command {index}: Move role {role:?} does not match the entity role {expected_role:?}"
                    ));
                }
                if e.is_some() != *has_e {
                    return Err(format!(
                        "command {index}: Move e presence {} does not match the documented rule ({}), e={e:?}",
                        if e.is_some() { "Some" } else { "None" },
                        if *has_e {
                            "positive-length follow-up point"
                        } else {
                            "first or zero-length point"
                        }
                    ));
                }
                if let Some(value) = e {
                    if !value.is_finite() {
                        return Err(format!("command {index}: Move e {value} is not finite"));
                    }
                    if let Some(previous) = last_e {
                        if !(*value > previous) {
                            return Err(format!(
                                "command {index}: cumulative E {value} did not increase over {previous}; a per-entity or reset accumulator was observed"
                            ));
                        }
                    }
                    last_e = Some(*value);
                }
            }
            (
                GCodeCommand::Retract {
                    length,
                    speed,
                    mode,
                },
                ModelCommand::Retract {
                    length: expected_length,
                    speed: expected_speed,
                    mode: expected_mode,
                },
            ) => {
                if length != expected_length || speed != expected_speed || mode != expected_mode {
                    return Err(format!(
                        "command {index}: Retract ({length}, {speed}, {mode:?}) does not match the declared side entry ({expected_length}, {expected_speed}, {expected_mode:?})"
                    ));
                }
            }
            (
                GCodeCommand::Unretract {
                    length,
                    speed,
                    mode,
                },
                ModelCommand::Unretract {
                    length: expected_length,
                    speed: expected_speed,
                    mode: expected_mode,
                },
            ) => {
                if length != expected_length || speed != expected_speed || mode != expected_mode {
                    return Err(format!(
                        "command {index}: Unretract ({length}, {speed}, {mode:?}) does not match the declared side entry ({expected_length}, {expected_speed}, {expected_mode:?})"
                    ));
                }
            }
            (
                GCodeCommand::ToolChange {
                    after_entity_index,
                    from,
                    to,
                },
                ModelCommand::ToolChange {
                    after_entity_index: expected_after,
                    from: expected_from,
                    to: expected_to,
                },
            ) => {
                if after_entity_index != expected_after
                    || from != expected_from
                    || to != expected_to
                {
                    return Err(format!(
                        "command {index}: ToolChange ({after_entity_index}, {from}->{to}) does not match the predicted sequence ({expected_after}, {expected_from}->{expected_to})"
                    ));
                }
            }
            (GCodeCommand::Comment { text }, ModelCommand::Comment { text: expected_text }) => {
                if text != expected_text {
                    return Err(format!(
                        "command {index}: Comment {text:?} does not match the anchored annotation {expected_text:?}"
                    ));
                }
            }
            (GCodeCommand::Raw { text }, ModelCommand::Raw { text: expected_text }) => {
                if text != expected_text {
                    return Err(format!(
                        "command {index}: Raw {text:?} does not match the predicted header/annotation {expected_text:?}"
                    ));
                }
            }
            (
                GCodeCommand::ExtrusionMode { absolute },
                ModelCommand::ExtrusionMode {
                    absolute: expected_absolute,
                },
            ) => {
                if absolute != expected_absolute {
                    return Err(format!(
                        "command {index}: ExtrusionMode {absolute} does not open the stream as predicted ({expected_absolute})"
                    ));
                }
            }
            (observed, expected) => {
                return Err(format!(
                    "command {index}: kind mismatch between {observed:?} and the predicted {expected:?}"
                ))
            }
        }
    }

    if ir.metadata.layer_count != model.layer_count {
        return Err(format!(
            "metadata.layer_count is {}, the state model requires {} emitted layers",
            ir.metadata.layer_count, model.layer_count
        ));
    }
    let expected_len = model.max_tool as usize + 1;
    if ir.metadata.filament_used_mm.len() != expected_len {
        return Err(format!(
            "metadata.filament_used_mm has {} entries, the state model requires {expected_len} (max_tool {} from the tools that own Move points)",
            ir.metadata.filament_used_mm.len(),
            model.max_tool
        ));
    }

    let mut sums = vec![0.0_f64; expected_len];
    let mut current_tool = 0_u32;
    let mut previous_e = 0.0_f64;
    for command in walk {
        match command {
            GCodeCommand::ToolChange { to, .. } => current_tool = *to,
            GCodeCommand::Move { e: Some(value), .. } => {
                let current = f64::from(*value);
                let delta = current - previous_e;
                if let Some(slot) = sums.get_mut(current_tool as usize) {
                    *slot += delta;
                }
                previous_e = current;
            }
            _ => {}
        }
    }
    for (tool, (observed, expected)) in ir
        .metadata
        .filament_used_mm
        .iter()
        .zip(sums.iter())
        .enumerate()
    {
        if !observed.is_finite() || *observed < 0.0 {
            return Err(format!(
                "metadata.filament_used_mm[{tool}] = {observed} must be finite and nonnegative"
            ));
        }
        let envelope = 8.0 * f64::from(f32::EPSILON) * (expected.abs() + previous_e.abs() + 1.0);
        if (f64::from(*observed) - expected).abs() > envelope {
            return Err(format!(
                "metadata.filament_used_mm[{tool}] = {observed} does not equal the stream's E delta sum {expected} within the derived f32 accumulation envelope {envelope}"
            ));
        }
    }
    for tool in 0..model.max_tool {
        if !model.tools_with_moves.contains(&tool)
            && ir.metadata.filament_used_mm[tool as usize] != 0.0
        {
            return Err(format!(
                "metadata.filament_used_mm[{tool}] is nonzero for a tool that owns no Move point"
            ));
        }
    }
    Ok(())
}

/// Emit a scene and return the model and the real IR together.
fn emission_model_and_ir(
    layers: &[slicer_ir::LayerCollectionIR],
) -> Result<(EmissionModel, GCodeIR), String> {
    let feedrate = emission_feedrate_config();
    let resolved = emission_resolved_config();
    let emitter =
        DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), feedrate.clone())
            .with_resolved_config(resolved.clone())
            .with_extrusion_mode(true);
    let ir = emitter
        .emit_gcode(layers)
        .map_err(|error| format!("emit_gcode failed: {error}"))?;
    let owned = emitter
        .emit_gcode_owned(layers.to_vec())
        .map_err(|error| format!("emit_gcode_owned failed: {error}"))?;
    if ir != owned {
        return Err("emit_gcode and emit_gcode_owned disagree on the same scene".to_string());
    }
    let model = build_emission_model(layers, &feedrate, &resolved);
    Ok((model, ir))
}

fn check_emission_layers(layers: &[slicer_ir::LayerCollectionIR]) -> Result<(), String> {
    let (model, ir) = emission_model_and_ir(layers)?;
    compare_emission_stream(&model, &ir)
}

fn check_emission_scene(scene: &EmitRawScene) -> Result<(), String> {
    let layers = build_emission_layers(scene);
    check_emission_layers(&layers)
}

/// One entity for deterministic witnesses.
fn witness_entity(
    entity_id: u64,
    layer_index: u32,
    role: ExtrusionRole,
    points: &[&[(f32, f32)]],
) -> slicer_ir::PrintEntity {
    let z = 0.25 * (layer_index as f32 + 1.0);
    let flattened: Vec<slicer_ir::Point3WithWidth> = points
        .iter()
        .flat_map(|slice| slice.iter().copied())
        .map(|(x, y)| slicer_ir::Point3WithWidth {
            x,
            y,
            z,
            width: 0.5,
            flow_factor: 1.0,
            ..Default::default()
        })
        .collect();
    slicer_ir::PrintEntity {
        entity_id,
        path: slicer_ir::ExtrusionPath3D {
            points: flattened,
            role: role.clone(),
            speed_factor: 1.0,
            order_lock: Some(0),
            ..slicer_sdk::test_support::fixtures::extrusion_path3d_base(role.clone())
        },
        role: role.clone(),
        region_key: slicer_ir::RegionKey {
            global_layer_index: layer_index,
            object_id: "property-gcode-object".to_string(),
            region_id: 0,
            variant_chain: property_variant_chain(),
        },
        tool_index: 0,
        ..slicer_sdk::test_support::fixtures::print_entity_base(role)
    }
}

/// Deterministic emission witnesses: empty scenes, the skipped-layer rule, the
/// side-entry-only observation, the chain-insensitivity witness, singleton
/// bookkeeping, tool-change-only tools, zero-length segments and the labelled
/// negative modes.
fn emission_deterministic_witnesses() -> Result<(), String> {
    // 1. Empty layer collection: only the mode line plus the statistics tail.
    check_emission_layers(&[])?;

    // 2. A skipped empty layer between two output layers: the second output
    //    layer's height uses the previous OUTPUT layer's z, not the skipped z.
    let layer_zero = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![witness_entity(
            1,
            0,
            ExtrusionRole::SparseInfill,
            &[&[(0.0, 0.0), (1.0, 0.0)]],
        )],
        ..Default::default()
    };
    let layer_one_empty = slicer_ir::LayerCollectionIR {
        global_layer_index: 1,
        z: 0.5,
        ..Default::default()
    };
    let layer_two = slicer_ir::LayerCollectionIR {
        global_layer_index: 2,
        z: 0.75,
        ordered_entities: vec![witness_entity(
            2,
            2,
            ExtrusionRole::SparseInfill,
            &[&[(0.0, 1.0), (1.0, 1.0)]],
        )],
        ..Default::default()
    };
    check_emission_layers(&[layer_zero.clone(), layer_one_empty, layer_two.clone()])?;
    let (_, ir) = emission_model_and_ir(&[layer_zero.clone(), layer_two])?;
    if ir.metadata.layer_count != 2 {
        return Err(format!(
            "two output layers must count 2, got {}",
            ir.metadata.layer_count
        ));
    }
    let heights: Vec<&str> = ir
        .commands
        .iter()
        .filter_map(|command| match command {
            GCodeCommand::Raw { text } if text.starts_with(";HEIGHT:") => Some(text.as_str()),
            _ => None,
        })
        .collect();
    if heights != [";HEIGHT:0.25", ";HEIGHT:0.5"] {
        return Err(format!(
            "the second output layer's ;HEIGHT: must be z2 - z0 = 0.5, got {heights:?}"
        ));
    }

    // 3. A layer carrying only annotations is a no-output layer: annotations
    //    alone do not open a layer and are not emitted.
    let annotation_only = slicer_ir::LayerCollectionIR {
        global_layer_index: 1,
        z: 0.5,
        annotations: vec![slicer_ir::LayerAnnotation {
            after_entity_index: 0,
            kind: slicer_ir::LayerAnnotationKind::Comment("annotation-only".to_string()),
        }],
        ..Default::default()
    };
    let (_, ir) = emission_model_and_ir(&[layer_zero.clone(), annotation_only])?;
    if ir.metadata.layer_count != 1 {
        return Err(format!(
            "an annotation-only layer must not increment layer_count, got {}",
            ir.metadata.layer_count
        ));
    }
    if ir.commands.iter().any(
        |command| matches!(command, GCodeCommand::Comment { text } if text == "annotation-only"),
    ) {
        return Err("an annotation inside a move-less layer must not be emitted".to_string());
    }

    // 4. A layer with only a side-table entry (a speed profile keyed to an
    //    entity that does not exist) is still move-less: no output is produced.
    //    This classifies the side-entry observation; it does not claim emission.
    let side_entry_only = slicer_ir::LayerCollectionIR {
        global_layer_index: 1,
        z: 0.5,
        speed_profiles: vec![slicer_ir::EntitySpeedProfile {
            entity_id: 999,
            factors: vec![0.5],
        }],
        ..Default::default()
    };
    let (_, ir) = emission_model_and_ir(&[side_entry_only])?;
    if ir.metadata.layer_count != 0 {
        return Err(format!(
            "a side-entry-only layer must not open a layer, got layer_count {}",
            ir.metadata.layer_count
        ));
    }

    // 5. A layer whose only output driver is a tool-change side entry emits its
    //    headers but no ToolChange (tool changes are emitted inside the entity
    //    loop). This is the exact boundary the model mirrors.
    let tool_change_only_layer = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        tool_changes: vec![slicer_ir::ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 2,
        }],
        ..Default::default()
    };
    let (_, ir) = emission_model_and_ir(&[tool_change_only_layer])?;
    if ir.metadata.layer_count != 1 {
        return Err("a tool-change-only layer still counts as an output layer".to_string());
    }
    if ir
        .commands
        .iter()
        .any(|command| matches!(command, GCodeCommand::ToolChange { .. }))
    {
        return Err(
            "a tool change anchored in a layer with no entities must not be emitted".to_string(),
        );
    }

    // 6. Chain-insensitivity: the #11-ordered chain is fixture input only.
    let chained = vec![layer_zero.clone()];
    let mut unchained = layer_zero;
    for entity in &mut unchained.ordered_entities {
        entity.region_key.variant_chain = Vec::new();
    }
    let (_, chained_ir) = emission_model_and_ir(&chained)?;
    let (_, unchained_ir) = emission_model_and_ir(&[unchained])?;
    if chained_ir != unchained_ir {
        return Err(
            "the nonempty #11-ordered RegionKey.variant_chain changed the command stream"
                .to_string(),
        );
    }

    // 7. A singleton entity contributes a zero filament entry for its tool and
    //    no E value on its only Move.
    let singleton = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![witness_entity(
            1,
            0,
            ExtrusionRole::SparseInfill,
            &[&[(0.5, 0.5)]],
        )],
        ..Default::default()
    };
    let (_, ir) = emission_model_and_ir(&[singleton])?;
    if ir.metadata.filament_used_mm.len() != 1
        || ir.metadata.filament_used_mm[0].to_bits() != 0.0_f32.to_bits()
    {
        return Err(format!(
            "a singleton entity must contribute one zero filament entry, got {:?}",
            ir.metadata.filament_used_mm
        ));
    }
    let singleton_move_e = ir.commands.iter().find_map(|command| match command {
        GCodeCommand::Move { e, .. } => Some(*e),
        _ => None,
    });
    if singleton_move_e != Some(None) {
        return Err(format!(
            "a singleton entity's only Move must have e = None, got {singleton_move_e:?}"
        ));
    }

    // 8. A tool that appears only in tool_changes must not extend the vector.
    let tool_change_only = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![witness_entity(
            1,
            0,
            ExtrusionRole::SparseInfill,
            &[&[(0.0, 0.0), (1.0, 0.0)]],
        )],
        tool_changes: vec![slicer_ir::ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 5,
        }],
        ..Default::default()
    };
    let (_, ir) = emission_model_and_ir(&[tool_change_only])?;
    if ir.metadata.filament_used_mm.len() != 1 {
        return Err(format!(
            "a tool that appears only in tool_changes must not extend filament_used_mm, got {:?}",
            ir.metadata.filament_used_mm
        ));
    }

    // 9. A zero-length interior segment carries e = None while the following
    //    positive-length segment carries e = Some.
    let zero_length = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![witness_entity(
            1,
            0,
            ExtrusionRole::SparseInfill,
            &[&[(0.0, 0.0), (0.0, 0.0), (2.0, 0.0)]],
        )],
        ..Default::default()
    };
    let (_, ir) = emission_model_and_ir(&[zero_length])?;
    let move_e: Vec<Option<f32>> = ir
        .commands
        .iter()
        .filter_map(|command| match command {
            GCodeCommand::Move { e, .. } => Some(*e),
            _ => None,
        })
        .collect();
    if move_e.len() != 3 || move_e[0].is_some() || move_e[1].is_some() || move_e[2].is_none() {
        return Err(format!(
            "zero-length interior segment must carry e = None between two positive segments, got {move_e:?}"
        ));
    }

    emission_negative_modes()
}

/// The three labelled negative modes from the charter, observed exactly.
fn emission_negative_modes() -> Result<(), String> {
    use slicer_gcode::GCodeEmitError;

    let feedrate = emission_feedrate_config();

    // (a) enable_prime_tower + tool change without a WipeTower entity.
    let wipe_resolved = slicer_ir::ResolvedConfig {
        disable_m73: true,
        enable_prime_tower: true,
        ..emission_resolved_config()
    };
    let layer = slicer_ir::LayerCollectionIR {
        global_layer_index: 3,
        z: 0.25,
        ordered_entities: vec![
            witness_entity(
                1,
                3,
                ExtrusionRole::SparseInfill,
                &[&[(0.0, 0.0), (1.0, 0.0)]],
            ),
            witness_entity(
                2,
                3,
                ExtrusionRole::SparseInfill,
                &[&[(2.0, 0.0), (3.0, 0.0)]],
            ),
        ],
        tool_changes: vec![slicer_ir::ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        ..Default::default()
    };
    let emitter =
        DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), feedrate.clone())
            .with_resolved_config(wipe_resolved)
            .with_extrusion_mode(true);
    match emitter.emit_gcode(&[layer]) {
        Err(GCodeEmitError::MissingToolchangePurge {
            layer_index: 3,
            tool_change_index: 0,
        }) => {}
        other => {
            return Err(format!(
                "wipe-tower tool change without a purge entity must be MissingToolchangePurge {{ layer_index: 3, tool_change_index: 0 }}, got {other:?}"
            ))
        }
    }

    // (b) A tool index at MAX_PLAUSIBLE_TOOLS on an extruding entity.
    let layer = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![slicer_ir::PrintEntity {
            tool_index: 1_024,
            ..witness_entity(
                1,
                0,
                ExtrusionRole::SparseInfill,
                &[&[(0.0, 0.0), (1.0, 0.0)]],
            )
        }],
        ..Default::default()
    };
    let emitter =
        DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), feedrate.clone())
            .with_resolved_config(emission_resolved_config())
            .with_extrusion_mode(true);
    match emitter.emit_gcode(&[layer]) {
        Err(GCodeEmitError::ToolIndexOutOfRange {
            tool: 1_024,
            max: 1_023,
        }) => {}
        other => {
            return Err(format!(
                "tool index 1024 must be ToolIndexOutOfRange {{ tool: 1024, max: 1023 }}, got {other:?}"
            ))
        }
    }

    // (c) Automatic-volumetric branch: zero base speed, then every documented
    //     finite/positive guard with its exact message prefix.
    let mut zero_base = emission_feedrate_config();
    zero_base.sparse_infill_speed = 0.0;
    let automatic_error = |width: f32,
                           flow: f32,
                           speed_factor: f32,
                           initial_layer_print_height: f64,
                           maximum: Option<f64>|
     -> Result<String, String> {
        let mut resolved = slicer_ir::ResolvedConfig {
            initial_layer_print_height,
            ..emission_resolved_config()
        };
        if let Some(maximum) = maximum {
            resolved.extensions.insert(
                "filament_max_volumetric_speed".to_string(),
                slicer_ir::ConfigValue::Float(maximum),
            );
        }
        let mut entity = witness_entity(
            1,
            0,
            ExtrusionRole::SparseInfill,
            &[&[(0.0, 0.0), (1.0, 0.0)]],
        );
        for point in &mut entity.path.points {
            point.width = width;
            point.flow_factor = flow;
        }
        entity.path.speed_factor = speed_factor;
        let layer = slicer_ir::LayerCollectionIR {
            global_layer_index: 0,
            z: 0.25,
            ordered_entities: vec![entity],
            ..Default::default()
        };
        let emitter =
            DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), zero_base.clone())
                .with_resolved_config(resolved)
                .with_extrusion_mode(true);
        match emitter.emit_gcode(&[layer]) {
            Err(slicer_gcode::GCodeEmitError::Emit(message)) => Ok(message),
            other => Err(format!(
                "the automatic-volumetric branch must fail closed with GCodeEmitError::Emit, got {other:?}"
            )),
        }
    };

    let cases: [(&str, Result<String, String>); 6] = [
        (
            "automatic volumetric speed: speed factor must be finite",
            automatic_error(0.4, 1.0, f32::NAN, 0.25, Some(8.0)),
        ),
        (
            "automatic volumetric speed: move width must be finite and positive",
            automatic_error(f32::INFINITY, 1.0, 1.0, 0.25, Some(8.0)),
        ),
        (
            "automatic volumetric speed: layer height must be finite and positive",
            automatic_error(0.4, 1.0, 1.0, f64::INFINITY, Some(8.0)),
        ),
        (
            "automatic volumetric speed: flow factor must be finite and positive",
            automatic_error(0.4, f32::NAN, 1.0, 0.25, Some(8.0)),
        ),
        (
            "automatic volumetric speed: derived speed overflows f32",
            automatic_error(f32::MIN_POSITIVE, f32::MIN_POSITIVE, 1.0, 0.25, Some(10.0)),
        ),
        (
            "automatic volumetric speed: derived feedrate must be finite and positive",
            automatic_error(1.0e-18, 1.0e-18, 1.0, 0.25, Some(10.0)),
        ),
    ];
    for (expected_prefix, observed) in cases {
        let message = observed?;
        if !message.starts_with(expected_prefix) {
            return Err(format!(
                "automatic-volumetric error `{message}` does not start with the documented `{expected_prefix}`"
            ));
        }
    }

    // A valid automatic witness must produce a finite positive F (never F0 or
    // FNaN): 8 mm3/s over 0.4 x 0.2 x 1.0 resolves to the canonical F6000.
    let mut resolved = emission_resolved_config();
    resolved.initial_layer_print_height = 0.2;
    resolved.extensions.insert(
        "filament_max_volumetric_speed".to_string(),
        slicer_ir::ConfigValue::Float(8.0),
    );
    let mut entity = witness_entity(
        1,
        0,
        ExtrusionRole::SparseInfill,
        &[&[(0.0, 0.0), (1.0, 0.0)]],
    );
    for point in &mut entity.path.points {
        point.width = 0.4;
        point.flow_factor = 1.0;
    }
    let layer = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![entity],
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("property-gcode".to_string(), zero_base)
        .with_resolved_config(resolved)
        .with_extrusion_mode(true);
    let ir = emitter
        .emit_gcode(&[layer])
        .map_err(|error| format!("the valid automatic-volumetric witness failed: {error}"))?;
    let rates: Vec<Option<f32>> = ir
        .commands
        .iter()
        .filter_map(|command| match command {
            GCodeCommand::Move { f, .. } => Some(*f),
            _ => None,
        })
        .collect();
    if rates.is_empty() {
        return Err("the automatic-volumetric witness produced no Move".to_string());
    }
    for rate in &rates {
        match rate {
            Some(value) if value.is_finite() && *value > 0.0 => {}
            other => {
                return Err(format!(
                    "the automatic-volumetric branch must not emit F0/FNaN, got {other:?}"
                ))
            }
        }
    }
    if rates[0].map(f32::to_bits) != Some(6_000.0_f32.to_bits()) {
        return Err(format!(
            "8 mm3/s over 0.4 x 0.2 x 1.0 must resolve to F6000, got {:?}",
            rates[0]
        ));
    }
    Ok(())
}

#[test]
fn proptest_emission_state_sequence() {
    let run =
        PropertyRun::new(EMIT_PROPERTY_ID, EMIT_TEST_NAME).expect("H-GCODE-EMIT registry context");
    let bounds = emit_bounds(&run).expect("H-GCODE-EMIT checked profile bounds");
    emission_deterministic_witnesses().expect("H-GCODE-EMIT deterministic witnesses");
    let strategy = emit_scene_strategy(bounds);
    run.check(&strategy, |scene, _guard| {
        check_emission_scene(&scene).map_err(TestCaseError::fail)
    })
    .expect("H-GCODE-EMIT command/state sequence");
}

/// A rich deterministic scene used by the mutation controls: two entities, a
/// retract, a z-hop, a travel, a tool change, an annotation, a singleton
/// entity, a tool-change-only tool and a skipped empty layer.
fn emission_control_layers() -> Vec<slicer_ir::LayerCollectionIR> {
    let entity_a = witness_entity(
        1,
        0,
        ExtrusionRole::SparseInfill,
        &[&[(0.0, 0.0), (2.0, 0.0)]],
    );
    let entity_b = witness_entity(
        2,
        0,
        ExtrusionRole::TopSolidInfill,
        &[&[(0.0, 1.0), (2.0, 1.0)]],
    );
    let layer_zero = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![entity_a, entity_b],
        tool_changes: vec![slicer_ir::ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        z_hops: vec![slicer_ir::ZHop {
            after_entity_index: 0,
            hop_height: 0.5,
        }],
        retracts: vec![
            // exhaustive: concrete anchor fixtures, every field intended.
            slicer_ir::TravelRetract {
                after_entity_index: 0,
                length: 1.0,
                speed: 40.0,
                is_unretract: false,
                mode: slicer_ir::RetractMode::Gcode,
            },
            // exhaustive: the matching unretract anchor fixture; all fields intended.
            slicer_ir::TravelRetract {
                after_entity_index: 0,
                length: 1.0,
                speed: 40.0,
                is_unretract: true,
                mode: slicer_ir::RetractMode::Gcode,
            },
        ],
        travel_moves: vec![slicer_ir::TravelMove {
            entity_id: 1,
            x: Some(5.0),
            y: Some(5.0),
            f: None,
            ..slicer_ir::TravelMove::default()
        }],
        annotations: vec![slicer_ir::LayerAnnotation {
            after_entity_index: 0,
            kind: slicer_ir::LayerAnnotationKind::Comment("after-a".to_string()),
        }],
        ..Default::default()
    };
    let layer_one = slicer_ir::LayerCollectionIR {
        global_layer_index: 1,
        z: 0.5,
        ordered_entities: vec![witness_entity(
            3,
            1,
            ExtrusionRole::SparseInfill,
            &[&[(3.0, 3.0)]],
        )],
        tool_changes: vec![slicer_ir::ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 5,
        }],
        ..Default::default()
    };
    let layer_two_empty = slicer_ir::LayerCollectionIR {
        global_layer_index: 2,
        z: 0.75,
        ..Default::default()
    };
    vec![layer_zero, layer_one, layer_two_empty]
}

/// Deterministic controls for `H-GCODE-EMIT`: the shared checker accepts the
/// real stream and rejects one mutation per named defect class, the negative
/// SUT modes observe the exact error variants, and the shrink/complicate witness
/// probe keeps the strategy honest.
#[test]
fn gcode_emission_controls() {
    emission_deterministic_witnesses().expect("H-GCODE-EMIT deterministic witnesses");

    let layers = emission_control_layers();
    let (model, ir) = emission_model_and_ir(&layers).expect("the control scene must emit");
    compare_emission_stream(&model, &ir).expect("the documented model must accept the real stream");

    // 1. A missing layer header.
    let mut mutated = ir.clone();
    let header = mutated
        .commands
        .iter()
        .position(
            |command| matches!(command, GCodeCommand::Raw { text } if text == ";LAYER_CHANGE"),
        )
        .expect("the stream must open a layer");
    mutated.commands.remove(header);
    assert!(
        compare_emission_stream(&model, &mutated).is_err(),
        "a missing ;LAYER_CHANGE header must be rejected"
    );

    // 2. A dropped empty-layer skip: headers for the skipped layer appear even
    //    though the model emitted nothing for it.
    let mut mutated = ir.clone();
    let insert_at = ir
        .commands
        .iter()
        .position(
            |command| matches!(command, GCodeCommand::Raw { text } if text.starts_with(";HEIGHT:")),
        )
        .expect("the stream must carry a ;HEIGHT: header")
        + 1;
    mutated.commands.insert(
        insert_at,
        GCodeCommand::Raw {
            text: ";LAYER_CHANGE".to_string(),
        },
    );
    mutated.commands.insert(
        insert_at + 1,
        GCodeCommand::Raw {
            text: ";Z:0.5".to_string(),
        },
    );
    mutated.commands.insert(
        insert_at + 2,
        GCodeCommand::Raw {
            text: ";HEIGHT:0.25".to_string(),
        },
    );
    assert!(
        compare_emission_stream(&model, &mutated).is_err(),
        "headers for a skipped empty layer must be rejected"
    );

    // 3. An unretracted tool change.
    let mut mutated = ir.clone();
    let tool_change = mutated
        .commands
        .iter()
        .position(|command| matches!(command, GCodeCommand::ToolChange { .. }))
        .expect("the control scene must carry a tool change");
    let retract = mutated.commands[..tool_change]
        .iter()
        .rposition(|command| matches!(command, GCodeCommand::Retract { .. }))
        .expect("the control scene must retract before the tool change");
    mutated.commands.remove(retract);
    assert!(
        compare_emission_stream(&model, &mutated).is_err(),
        "an unretracted tool change must be rejected"
    );

    // 4. A reordered retract/z-hop pair.
    let mut mutated = ir.clone();
    let retract = mutated
        .commands
        .iter()
        .position(|command| matches!(command, GCodeCommand::Retract { .. }))
        .expect("the control scene must retract");
    let z_hop_up = mutated.commands[retract + 1..]
        .iter()
        .position(|command| {
            matches!(
                command,
                GCodeCommand::Move {
                    z: Some(_),
                    role,
                    ..
                } if matches!(role, ExtrusionRole::Custom(name) if name.as_str() == "Travel")
            )
        })
        .map(|offset| retract + 1 + offset)
        .expect("the control scene must z-hop after the retract");
    mutated.commands.swap(retract, z_hop_up);
    assert!(
        compare_emission_stream(&model, &mutated).is_err(),
        "a reordered retract/z-hop sequence must be rejected"
    );

    // 5. A per-entity rather than whole-scene E accumulator: the second
    //    extruding Move restarts near zero instead of continuing the print.
    let mut mutated = ir.clone();
    let extruding: Vec<usize> = mutated
        .commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| match command {
            GCodeCommand::Move { e: Some(_), .. } => Some(index),
            _ => None,
        })
        .collect();
    assert!(
        extruding.len() >= 2,
        "the control scene must expose at least two extruding Moves"
    );
    let second = extruding[1];
    let previous = match &mutated.commands[extruding[0]] {
        GCodeCommand::Move { e: Some(value), .. } => *value,
        other => panic!("expected an extruding Move, got {other:?}"),
    };
    if let GCodeCommand::Move { e, .. } = &mut mutated.commands[second] {
        *e = Some(previous * 0.5);
    }
    assert!(
        compare_emission_stream(&model, &mutated).is_err(),
        "a per-entity E accumulator must be rejected"
    );

    // 6. A dropped zero filament entry for a singleton entity.
    let singleton = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![witness_entity(
            1,
            0,
            ExtrusionRole::SparseInfill,
            &[&[(0.5, 0.5)]],
        )],
        ..Default::default()
    };
    let (singleton_model, singleton_ir) =
        emission_model_and_ir(&[singleton]).expect("the singleton scene must emit");
    compare_emission_stream(&singleton_model, &singleton_ir)
        .expect("the singleton zero entry must be accepted");
    let mut mutated = singleton_ir.clone();
    mutated.metadata.filament_used_mm.clear();
    assert!(
        compare_emission_stream(&singleton_model, &mutated).is_err(),
        "a dropped singleton zero entry must be rejected"
    );

    // 7. A fabricated entry for a tool that only appears in tool_changes.
    let tool_change_only = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.25,
        ordered_entities: vec![witness_entity(
            1,
            0,
            ExtrusionRole::SparseInfill,
            &[&[(0.0, 0.0), (1.0, 0.0)]],
        )],
        tool_changes: vec![slicer_ir::ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 5,
        }],
        ..Default::default()
    };
    let (tool_change_model, tool_change_ir) =
        emission_model_and_ir(&[tool_change_only]).expect("the tool-change-only scene must emit");
    compare_emission_stream(&tool_change_model, &tool_change_ir)
        .expect("a tool-change-only tool must not extend the filament vector");
    let mut mutated = tool_change_ir.clone();
    mutated.metadata.filament_used_mm.push(0.0);
    assert!(
        compare_emission_stream(&tool_change_model, &mutated).is_err(),
        "a fabricated tool-change-only filament entry must be rejected"
    );

    // Non-vacuous shrink/complicate probe over the same strategy and checker.
    // The emission generator has no `no_shrink` persistent mode: every field of
    // a scene may legitimately simplify away, so this probe asserts validity
    // after each protocol step and does not fabricate a mode-equality claim.
    let strategy = emit_scene_strategy(CONTROL_EMIT_BOUNDS);
    shrink_complicate_witness(
        EMIT_PROPERTY_ID,
        &strategy,
        |_| None,
        |scene| check_emission_scene(scene),
    );
}

// ════════════════════════════════════════════════════════════════════════════
// H-GCODE-SERIALIZE: an independent text and extrusion-state model
// ════════════════════════════════════════════════════════════════════════════

/// Registry identity of the serializer charter.
const SERIALIZE_PROPERTY_ID: &str = "H-GCODE-SERIALIZE";
/// Exact registry test name of the serializer charter.
const SERIALIZE_TEST_NAME: &str = "proptest_serializer_extrusion_state";

/// Stream-level extrusion mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SerializeMode {
    Relative,
    Absolute,
}

impl SerializeMode {
    fn is_relative(self) -> bool {
        matches!(self, SerializeMode::Relative)
    }

    fn mode_line(self) -> &'static str {
        match self {
            SerializeMode::Relative => "M83",
            SerializeMode::Absolute => "M82",
        }
    }
}

/// The two temperature forms the charter exercises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SerializeFlavor {
    Marlin,
    RepRapFirmware,
}

impl SerializeFlavor {
    fn flavor(self) -> GcodeFlavor {
        match self {
            SerializeFlavor::Marlin => GcodeFlavor::Marlin,
            SerializeFlavor::RepRapFirmware => GcodeFlavor::RepRapFirmware,
        }
    }
}

/// Retract emission mode in the generated command list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SerRetractMode {
    Gcode,
    Firmware,
}

impl SerRetractMode {
    fn mode(self) -> slicer_ir::RetractMode {
        match self {
            SerRetractMode::Gcode => slicer_ir::RetractMode::Gcode,
            SerRetractMode::Firmware => slicer_ir::RetractMode::Firmware,
        }
    }
}

/// One generated command with exact integer grid units.
///
/// Grids: XY `1/8` mm (3 decimals), Z `1/4` mm (2 decimals), E and retract
/// lengths `1/32` mm (5 decimals), F and retract speeds `1/16` (4 decimals).
/// Every scaled value is therefore an exact integer and the decimal oracle has
/// no rounding ambiguity on the declared domain.
#[derive(Debug, Clone, PartialEq)]
enum SerCommand {
    Move {
        x_units: i64,
        y_units: i64,
        z_units: Option<i64>,
        e_units: Option<i64>,
        f_units: Option<i64>,
        travel: bool,
    },
    Retract {
        length_units: i64,
        speed_units: i64,
        mode: SerRetractMode,
    },
    Unretract {
        length_units: i64,
        speed_units: i64,
        mode: SerRetractMode,
    },
    Fan(u8),
    Temperature {
        tool: u32,
        celsius: u16,
        wait: bool,
    },
    ToolChange(u32),
    Comment(String),
    Raw(String),
}

/// One generated serializer stream.
#[derive(Debug, Clone, PartialEq)]
struct SerializeCase {
    mode: SerializeMode,
    flavor: SerializeFlavor,
    commands: Vec<SerCommand>,
}

/// Checked registry bounds for the serializer charter.
#[derive(Debug, Clone, Copy)]
struct SerializeBounds {
    max_commands: usize,
    max_abs_units: i64,
    max_e_units: i64,
}

/// Test-local profile mirroring the registry's measured routine bounds.
const CONTROL_SERIALIZE_BOUNDS: SerializeBounds = SerializeBounds {
    max_commands: 12,
    max_abs_units: 512,
    max_e_units: 3_200,
};

fn serialize_bounds(run: &PropertyRun) -> Result<SerializeBounds, String> {
    let max_commands = usize::try_from(positive_bound(run, "max_commands")?)
        .map_err(|error| format!("bound max_commands: {error}"))?;
    let max_abs_mm = i64::try_from(positive_bound(run, "max_abs_mm")?)
        .map_err(|error| format!("bound max_abs_mm: {error}"))?;
    let max_abs_e_mm = i64::try_from(positive_bound(run, "max_abs_e_mm")?)
        .map_err(|error| format!("bound max_abs_e_mm: {error}"))?;
    let max_abs_units = max_abs_mm
        .checked_mul(8)
        .ok_or_else(|| format!("bound max_abs_mm {max_abs_mm} overflows 1/8 mm units"))?;
    let max_e_units = max_abs_e_mm
        .checked_mul(32)
        .ok_or_else(|| format!("bound max_abs_e_mm {max_abs_e_mm} overflows 1/32 mm units"))?;
    if max_commands > 64 {
        return Err(format!(
            "serializer profile max_commands {max_commands} exceeds the 64-command allocation guard"
        ));
    }
    if max_abs_units <= 0 || max_e_units <= 0 {
        return Err("serializer bounds must yield positive grid limits".to_string());
    }
    Ok(SerializeBounds {
        max_commands,
        max_abs_units,
        max_e_units,
    })
}

/// Fixed raw-line shapes required by the charter.
const RAW_LINE_SHAPES: [&str; 7] = [
    "G92 E0",
    "G92 E1.5",
    "G92 X0",
    "G92 E",
    "M104 S200",
    "G92 E12.25",
    "G1 X0",
];

fn ser_command_strategy(bounds: SerializeBounds) -> impl Strategy<Value = SerCommand> {
    let limit = bounds.max_abs_units;
    let e_limit = bounds.max_e_units;
    let move_command = (
        -limit..=limit,
        -limit..=limit,
        prop::option::of(-limit..=limit),
        prop::option::of(0..=e_limit),
        prop::option::of(0..=96_000_i64),
        select(vec![false, true]).no_shrink(),
    )
        .prop_map(
            |(x_units, y_units, z_units, e_units, f_units, travel)| SerCommand::Move {
                x_units,
                y_units,
                z_units,
                e_units,
                f_units,
                travel,
            },
        );
    let retract = (
        0..=e_limit,
        0..=4_800_i64,
        select(vec![SerRetractMode::Gcode, SerRetractMode::Firmware]).no_shrink(),
    )
        .prop_map(|(length_units, speed_units, mode)| SerCommand::Retract {
            length_units,
            speed_units,
            mode,
        });
    let unretract = (
        0..=e_limit,
        0..=4_800_i64,
        select(vec![SerRetractMode::Gcode, SerRetractMode::Firmware]).no_shrink(),
    )
        .prop_map(|(length_units, speed_units, mode)| SerCommand::Unretract {
            length_units,
            speed_units,
            mode,
        });
    prop_oneof![
        10 => move_command,
        3 => retract,
        3 => unretract,
        2 => (0..=255_u8).prop_map(SerCommand::Fan),
        2 => (0..=3_u32, 0..=300_u16, select(vec![false, true]).no_shrink()).prop_map(
            |(tool, celsius, wait)| SerCommand::Temperature { tool, celsius, wait }
        ),
        2 => (0..=3_u32).prop_map(SerCommand::ToolChange),
        2 => property_ascii_text().prop_map(SerCommand::Comment),
        3 => prop_oneof![
            6 => select(RAW_LINE_SHAPES.to_vec()).no_shrink().prop_map(|line| SerCommand::Raw(line.to_string())),
            2 => property_ascii_text().prop_map(SerCommand::Raw),
        ],
    ]
}

fn serialize_case_strategy(bounds: SerializeBounds) -> impl Strategy<Value = SerializeCase> {
    (
        select(vec![SerializeMode::Relative, SerializeMode::Absolute]).no_shrink(),
        select(vec![
            SerializeFlavor::Marlin,
            SerializeFlavor::RepRapFirmware,
        ])
        .no_shrink(),
        prop::collection::vec(ser_command_strategy(bounds), 0..=bounds.max_commands),
    )
        .prop_map(|(mode, flavor, commands)| SerializeCase {
            mode,
            flavor,
            commands,
        })
}

/// Build the exact `GCodeIR` command list a case denotes. The single leading
/// `ExtrusionMode` matches the serializer's mode; no later mode switch is
/// generated.
fn serializer_commands(case: &SerializeCase) -> Vec<GCodeCommand> {
    let mut commands = Vec::with_capacity(case.commands.len() + 1);
    commands.push(GCodeCommand::ExtrusionMode {
        absolute: case.mode == SerializeMode::Absolute,
    });
    for command in &case.commands {
        commands.push(match command {
            SerCommand::Move {
                x_units,
                y_units,
                z_units,
                e_units,
                f_units,
                travel,
            } => GCodeCommand::Move {
                x: Some(*x_units as f32 / 8.0),
                y: Some(*y_units as f32 / 8.0),
                z: z_units.map(|units| units as f32 / 4.0),
                e: e_units.map(|units| units as f32 / 32.0),
                f: f_units.map(|units| units as f32 / 16.0),
                role: if *travel {
                    ExtrusionRole::Custom("Travel".to_string())
                } else {
                    ExtrusionRole::OuterWall
                },
            },
            SerCommand::Retract {
                length_units,
                speed_units,
                mode,
            } => GCodeCommand::Retract {
                length: *length_units as f32 / 32.0,
                speed: *speed_units as f32 / 16.0,
                mode: mode.mode(),
            },
            SerCommand::Unretract {
                length_units,
                speed_units,
                mode,
            } => GCodeCommand::Unretract {
                length: *length_units as f32 / 32.0,
                speed: *speed_units as f32 / 16.0,
                mode: mode.mode(),
            },
            SerCommand::Fan(value) => GCodeCommand::FanSpeed { value: *value },
            SerCommand::Temperature {
                tool,
                celsius,
                wait,
            } => GCodeCommand::Temperature {
                tool: *tool,
                celsius: f32::from(*celsius),
                wait: *wait,
            },
            SerCommand::ToolChange(to) => GCodeCommand::ToolChange {
                after_entity_index: 0,
                from: 0,
                to: *to,
            },
            SerCommand::Comment(text) => GCodeCommand::Comment { text: text.clone() },
            SerCommand::Raw(text) => GCodeCommand::Raw { text: text.clone() },
        });
    }
    commands
}

fn serializer_metadata() -> slicer_ir::PrintMetadata {
    slicer_ir::PrintMetadata {
        estimated_print_time_s: 0,
        filament_used_mm: vec![0.0],
        layer_count: 1,
        slicer_version: "property-gcode".to_string(),
    }
}

/// One named way the documented serializer contract can be wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SerializerMutation {
    /// The documented contract.
    None,
    /// Swap the relative/absolute E policy while leaving the mode line alone.
    ModeSwap,
    /// Ignore `G92 E<value>` accumulator resets.
    IgnoreG92,
    /// Reproduce the former absolute-mode defect (bare delta text, no update)
    /// and the relative-mode "emit the delta but do not update" error.
    AccumulatorLost,
    /// Never trim trailing zeros from the coordinate formatter.
    UntrimmedFormatting,
    /// Emit the two header lines in the wrong order.
    ReorderedHeader,
    /// Perturb each emitted E value by one 1e-5 grid step.
    MutatedDelta,
}

/// The rule a `G92` raw line follows: the accumulator changes iff the first
/// `E`-prefixed whitespace token parses as an `f64`.
fn oracle_parse_g92_e(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if !trimmed.starts_with("G92") {
        return None;
    }
    let token = trimmed
        .split_whitespace()
        .find(|token| token.starts_with('E') || token.starts_with('e'))?;
    token[1..].parse::<f64>().ok()
}

/// Render the exact header block plus width comments.
fn oracle_header(max_z_mm: f32, mutation: SerializerMutation) -> String {
    let max_z = format_scaled_fixed((f64::from(max_z_mm) * 100.0).round() as i128, 2);
    let total_layers = "; total layer number: 1".to_string();
    let diameter = "; filament_diameter: 1.75".to_string();
    let (first, second) = if mutation == SerializerMutation::ReorderedHeader {
        (diameter, total_layers)
    } else {
        (total_layers, diameter)
    };
    let mut out = String::new();
    out.push_str("; HEADER_BLOCK_START\n");
    out.push_str("; OrcaSlicer-compatible output generated by Pinch'n'Print\n");
    out.push_str(&first);
    out.push('\n');
    out.push_str(&second);
    out.push('\n');
    out.push_str("; filament_density: 1.24\n");
    out.push_str(&format!("; max_z_height: {max_z}\n"));
    out.push_str("; filament: 1\n");
    out.push_str("; filament_colour = #EC0006\n");
    out.push_str("; extruder_colour = #EC0006\n");
    out.push_str("; HEADER_BLOCK_END\n");
    out.push_str("; outer_wall_line_width = 0.4\n");
    out.push_str("; inner_wall_line_width = 0.4\n");
    out.push_str("; sparse_infill_line_width = 0.45\n");
    out.push_str("; top_surface_line_width = 0.42\n");
    out.push_str("; support_line_width = 0.4\n");
    out
}

/// Independent renderer for the documented text and extrusion-state contract.
fn render_serializer_oracle(
    case: &SerializeCase,
    commands: &[GCodeCommand],
    mutation: SerializerMutation,
) -> String {
    let mut max_z: f32 = 0.0;
    for command in commands {
        if let GCodeCommand::Move { z: Some(value), .. } = command {
            max_z = max_z.max(*value);
        }
    }
    let max_z = if max_z > 0.0 { max_z } else { 256.0 };
    let mut out = oracle_header(max_z, mutation);

    let mut relative = case.mode.is_relative();
    if mutation == SerializerMutation::ModeSwap {
        relative = !relative;
    }
    let mut accumulator: f64 = 0.0;
    let coordinate = |value: f32| {
        if mutation == SerializerMutation::UntrimmedFormatting {
            format_scaled_fixed((f64::from(value) * 1000.0).round() as i128, 3)
        } else {
            oracle_coord(value, 3)
        }
    };

    for command in commands {
        match command {
            GCodeCommand::Move {
                x,
                y,
                z,
                e,
                f,
                role,
            } => {
                let is_travel =
                    matches!(role, ExtrusionRole::Custom(name) if name.as_str() == "Travel");
                out.push_str(if is_travel { "G0" } else { "G1" });
                if let Some(value) = x {
                    out.push_str(" X");
                    out.push_str(&coordinate(*value));
                }
                if let Some(value) = y {
                    out.push_str(" Y");
                    out.push_str(&coordinate(*value));
                }
                if let Some(value) = z {
                    out.push_str(" Z");
                    out.push_str(&coordinate(*value));
                }
                if let Some(value) = e {
                    out.push_str(" E");
                    if relative {
                        let delta = f64::from(*value) - accumulator;
                        let delta = if mutation == SerializerMutation::MutatedDelta {
                            delta + 0.000_01
                        } else {
                            delta
                        };
                        out.push_str(&oracle_e5(delta));
                        accumulator = f64::from(*value);
                    } else {
                        let emitted = if mutation == SerializerMutation::MutatedDelta {
                            f64::from(*value) + 0.000_01
                        } else {
                            f64::from(*value)
                        };
                        out.push_str(&oracle_e5(emitted));
                        accumulator = f64::from(*value);
                    }
                }
                if let Some(value) = f {
                    out.push_str(" F");
                    out.push_str(&oracle_coord(*value, 4));
                }
                out.push('\n');
            }
            GCodeCommand::Retract {
                length,
                speed,
                mode,
            } => match mode {
                slicer_ir::RetractMode::Firmware => out.push_str("G10\n"),
                slicer_ir::RetractMode::Gcode => {
                    if relative {
                        out.push_str("G1 E-");
                        out.push_str(&oracle_e5(f64::from(*length)));
                        if mutation != SerializerMutation::AccumulatorLost {
                            accumulator -= f64::from(*length);
                        }
                    } else if mutation == SerializerMutation::AccumulatorLost {
                        // The former absolute branch: a bare delta at four
                        // trimmed decimals and no accumulator update.
                        out.push_str("G1 E-");
                        out.push_str(&oracle_coord(*length, 4));
                    } else {
                        accumulator -= f64::from(*length);
                        out.push_str("G1 E");
                        out.push_str(&oracle_e5(accumulator));
                    }
                    out.push_str(" F");
                    out.push_str(&oracle_coord(*speed, 4));
                    out.push('\n');
                }
            },
            GCodeCommand::Unretract {
                length,
                speed,
                mode,
            } => match mode {
                slicer_ir::RetractMode::Firmware => out.push_str("G11\n"),
                slicer_ir::RetractMode::Gcode => {
                    if relative {
                        out.push_str("G1 E");
                        out.push_str(&oracle_e5(f64::from(*length)));
                        if mutation != SerializerMutation::AccumulatorLost {
                            accumulator += f64::from(*length);
                        }
                    } else if mutation == SerializerMutation::AccumulatorLost {
                        out.push_str("G1 E");
                        out.push_str(&oracle_coord(*length, 4));
                    } else {
                        accumulator += f64::from(*length);
                        out.push_str("G1 E");
                        out.push_str(&oracle_e5(accumulator));
                    }
                    out.push_str(" F");
                    out.push_str(&oracle_coord(*speed, 4));
                    out.push('\n');
                }
            },
            GCodeCommand::Raw { text } => {
                if mutation != SerializerMutation::IgnoreG92 {
                    if let Some(value) = oracle_parse_g92_e(text) {
                        accumulator = value;
                    }
                }
                out.push_str(text);
                out.push('\n');
            }
            GCodeCommand::FanSpeed { value } => {
                out.push_str(&format!("M106 S{value}\n"));
            }
            GCodeCommand::Temperature {
                tool,
                celsius,
                wait,
            } => match case.flavor {
                SerializeFlavor::Marlin => {
                    out.push_str(&format!(
                        "M{} T{} S{}\n",
                        if *wait { 109 } else { 104 },
                        tool,
                        *celsius
                    ));
                }
                SerializeFlavor::RepRapFirmware => {
                    out.push_str(&format!("G10 P{} S{}\n", tool, *celsius));
                    if *wait {
                        out.push_str("M116\n");
                    }
                }
            },
            GCodeCommand::ToolChange { to, .. } => {
                out.push_str(&format!("T{to}\n"));
            }
            GCodeCommand::Comment { text } => {
                out.push_str("; ");
                out.push_str(text);
                out.push('\n');
            }
            GCodeCommand::ExtrusionMode { absolute } => {
                out.push_str(if *absolute { "M82\n" } else { "M83\n" });
            }
        }
    }
    out
}

/// Describe the first differing line, with a short context window.
fn first_line_difference(observed: &str, expected: &str) -> String {
    let observed_lines: Vec<&str> = observed.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    for index in 0..observed_lines.len().max(expected_lines.len()) {
        let observed_line = observed_lines.get(index).copied().unwrap_or("<missing>");
        let expected_line = expected_lines.get(index).copied().unwrap_or("<missing>");
        if observed_line != expected_line {
            let start = index.saturating_sub(3);
            let end = (index + 3).min(observed_lines.len().max(expected_lines.len()));
            let context: Vec<String> = (start..end)
                .map(|line| {
                    format!(
                        "{line}: observed {:?} | oracle {:?}",
                        observed_lines.get(line).copied().unwrap_or("<missing>"),
                        expected_lines.get(line).copied().unwrap_or("<missing>")
                    )
                })
                .collect();
            return format!(
                "line {index}: observed {observed_line:?}, oracle {expected_line:?}\n{}",
                context.join("\n")
            );
        }
    }
    format!(
        "the texts differ in length: observed {} bytes, oracle {} bytes",
        observed.len(),
        expected.len()
    )
}

/// Run the real serializer and compare its exact text to the independent model.
fn check_serializer_case(case: &SerializeCase, mutation: SerializerMutation) -> Result<(), String> {
    let commands = serializer_commands(case);
    let ir = GCodeIR {
        commands: commands.clone(),
        metadata: serializer_metadata(),
        ..Default::default()
    };
    let serializer = DefaultGCodeSerializer::with_extrusion_mode(case.mode.is_relative())
        .with_flavor(case.flavor.flavor());
    let observed = serializer
        .serialize_gcode(&ir)
        .map_err(|error| format!("serialize_gcode failed: {error}"))?;
    let expected = render_serializer_oracle(case, &commands, mutation);
    if observed != expected {
        return Err(format!(
            "serializer text differs from the independent renderer ({mutation:?}):\n{}",
            first_line_difference(&observed, &expected)
        ));
    }
    if mutation == SerializerMutation::None {
        let mode_lines: Vec<&str> = observed
            .lines()
            .filter(|line| *line == "M82" || *line == "M83")
            .collect();
        if mode_lines != [case.mode.mode_line()] {
            return Err(format!(
                "the stream must open with exactly one {} line, got {mode_lines:?}",
                case.mode.mode_line()
            ));
        }
    }
    Ok(())
}

/// Pin the formatter's actual tie and signed-zero behaviour before the oracle
/// trusts its own decimal rendering. This is a probe, not a claim that the
/// production choice is canonical.
fn serializer_formatting_probe() -> Result<(), String> {
    // Exact dyadic values: every declared grid value formats without a tie.
    for (value, expected) in [
        (1.125_f32, "1.125"),
        (-1.125_f32, "-1.125"),
        (64.0_f32, "64"),
        (0.0_f32, "0"),
    ] {
        let observed = format_xyz(value, 3);
        if observed != expected {
            return Err(format!(
                "format_xyz({value}, 3) = {observed:?}, the oracle requires {expected:?}"
            ));
        }
        let rendered = oracle_coord(value, 3);
        if rendered != expected {
            return Err(format!(
                "the independent 3-decimal model renders {value} as {rendered:?}, not {expected:?}"
            ));
        }
    }

    // Signed zero: the production fast path keeps the sign, so the oracle must
    // too (`-0.0` renders as `-0`, never `0`).
    for rendered in [format_xyz(-0.0_f32, 3), format_coord(-0.0_f32)] {
        if rendered != "-0" {
            return Err(format!("-0.0 must render as \"-0\", observed {rendered:?}"));
        }
    }
    if oracle_coord(-0.0, 3) != "-0" {
        return Err("-0.0 must render as \"-0\" in the independent model".to_string());
    }

    // True decimal ties fall outside the declared 1/8 mm grid; the production
    // formatter's documented fallback rounds them with `core::fmt` (half to
    // even). The probe pins the observed behaviour so a future formatter change
    // is a deliberate, visible edit rather than a silent oracle drift.
    let tie = format_xyz(0.0625_f32, 3);
    if tie != "0.062" {
        return Err(format!(
            "format_xyz(0.0625, 3) = {tie:?}; the pinned tie behaviour is half-to-even (\"0.062\")"
        ));
    }

    // The 5-decimal E shape is fixed-width, not trimmed, and preserves -0.
    if oracle_e5(2.5) != "2.50000" {
        return Err("the 5-decimal E renderer must not trim trailing zeros".to_string());
    }
    if oracle_e5(-0.0) != "-0.00000" {
        return Err("the 5-decimal E renderer must keep the signed-zero sign".to_string());
    }
    Ok(())
}

/// Deterministic serializer witnesses: each command variant in both modes and
/// flavors, the fixed absolute-retract contract, firmware retract state, and
/// the formatter probe.
fn serializer_deterministic_witnesses() -> Result<(), String> {
    serializer_formatting_probe()?;

    let every_variant = SerializeCase {
        mode: SerializeMode::Relative,
        flavor: SerializeFlavor::Marlin,
        commands: vec![
            SerCommand::Move {
                x_units: 9,
                y_units: -8,
                z_units: Some(1),
                e_units: Some(32),
                f_units: Some(4_800),
                travel: false,
            },
            SerCommand::Move {
                x_units: 16,
                y_units: 8,
                z_units: None,
                e_units: None,
                f_units: None,
                travel: true,
            },
            SerCommand::Retract {
                length_units: 16,
                speed_units: 640,
                mode: SerRetractMode::Gcode,
            },
            SerCommand::Unretract {
                length_units: 16,
                speed_units: 640,
                mode: SerRetractMode::Gcode,
            },
            SerCommand::Retract {
                length_units: 16,
                speed_units: 640,
                mode: SerRetractMode::Firmware,
            },
            SerCommand::Unretract {
                length_units: 16,
                speed_units: 640,
                mode: SerRetractMode::Firmware,
            },
            SerCommand::Fan(255),
            SerCommand::Temperature {
                tool: 1,
                celsius: 200,
                wait: false,
            },
            SerCommand::Temperature {
                tool: 0,
                celsius: 215,
                wait: true,
            },
            SerCommand::ToolChange(2),
            SerCommand::Comment("hello".to_string()),
            SerCommand::Raw("M104 S200".to_string()),
        ],
    };
    for flavor in [SerializeFlavor::Marlin, SerializeFlavor::RepRapFirmware] {
        for mode in [SerializeMode::Relative, SerializeMode::Absolute] {
            let case = SerializeCase {
                mode,
                flavor,
                commands: every_variant.commands.clone(),
            };
            check_serializer_case(&case, SerializerMutation::None)?;
        }
    }

    // Every G92 shape, in both modes. `G92 E0`/`G92 E1.5`/`G92 E12.25` reset the
    // accumulator; `G92 X0`, `G92 E` and the non-G92 line do not.
    for mode in [SerializeMode::Relative, SerializeMode::Absolute] {
        let commands = RAW_LINE_SHAPES
            .iter()
            .map(|line| SerCommand::Raw(line.to_string()))
            .chain([
                SerCommand::Move {
                    x_units: 0,
                    y_units: 0,
                    z_units: None,
                    e_units: Some(64),
                    f_units: None,
                    travel: false,
                },
                SerCommand::Retract {
                    length_units: 8,
                    speed_units: 640,
                    mode: SerRetractMode::Gcode,
                },
            ])
            .collect();
        check_serializer_case(
            &SerializeCase {
                mode,
                flavor: SerializeFlavor::Marlin,
                commands,
            },
            SerializerMutation::None,
        )?;
    }

    // The absolute-mode contract: a Gcode retract/unretract emits the updated
    // absolute E and updates the accumulator, and a G92 reset applies in
    // absolute mode too.
    for mode in [SerializeMode::Relative, SerializeMode::Absolute] {
        let case = SerializeCase {
            mode,
            flavor: SerializeFlavor::Marlin,
            commands: vec![
                SerCommand::Move {
                    x_units: 8,
                    y_units: 16,
                    z_units: Some(1),
                    e_units: Some(64),
                    f_units: Some(48_000),
                    travel: false,
                },
                SerCommand::Retract {
                    length_units: 16,
                    speed_units: 640,
                    mode: SerRetractMode::Gcode,
                },
                SerCommand::Unretract {
                    length_units: 16,
                    speed_units: 640,
                    mode: SerRetractMode::Gcode,
                },
                SerCommand::Move {
                    x_units: 16,
                    y_units: 16,
                    z_units: Some(1),
                    e_units: Some(96),
                    f_units: Some(48_000),
                    travel: false,
                },
                SerCommand::Raw("G92 E0".to_string()),
                SerCommand::Retract {
                    length_units: 16,
                    speed_units: 640,
                    mode: SerRetractMode::Gcode,
                },
                SerCommand::Unretract {
                    length_units: 16,
                    speed_units: 640,
                    mode: SerRetractMode::Gcode,
                },
                SerCommand::Move {
                    x_units: 24,
                    y_units: 16,
                    z_units: Some(1),
                    e_units: Some(4),
                    f_units: Some(48_000),
                    travel: false,
                },
            ],
        };
        check_serializer_case(&case, SerializerMutation::None)?;
    }

    // Firmware retracts do not change the accumulator in either mode: the next
    // delta is exactly one grid step.
    for mode in [SerializeMode::Relative, SerializeMode::Absolute] {
        let case = SerializeCase {
            mode,
            flavor: SerializeFlavor::RepRapFirmware,
            commands: vec![
                SerCommand::Move {
                    x_units: 0,
                    y_units: 0,
                    z_units: None,
                    e_units: Some(32),
                    f_units: None,
                    travel: false,
                },
                SerCommand::Retract {
                    length_units: 64,
                    speed_units: 640,
                    mode: SerRetractMode::Firmware,
                },
                SerCommand::Unretract {
                    length_units: 64,
                    speed_units: 640,
                    mode: SerRetractMode::Firmware,
                },
                SerCommand::Move {
                    x_units: 8,
                    y_units: 0,
                    z_units: None,
                    e_units: Some(33),
                    f_units: None,
                    travel: false,
                },
            ],
        };
        check_serializer_case(&case, SerializerMutation::None)?;
        let commands = serializer_commands(&case);
        let ir = GCodeIR {
            commands,
            metadata: serializer_metadata(),
            ..Default::default()
        };
        let output = DefaultGCodeSerializer::with_extrusion_mode(mode.is_relative())
            .with_flavor(GcodeFlavor::RepRapFirmware)
            .serialize_gcode(&ir)
            .map_err(|error| format!("firmware witness failed: {error}"))?;
        let expected_delta = if mode.is_relative() {
            "G1 X1 Y0 E0.03125"
        } else {
            "G1 X1 Y0 E1.03125"
        };
        if !output.lines().any(|line| line == expected_delta) {
            return Err(format!(
                "a firmware retract must not change the accumulator; expected `{expected_delta}`:\n{output}"
            ));
        }
        if output.lines().any(|line| line.starts_with("G1 E")) {
            return Err(format!(
                "a firmware retract must emit G10/G11, never an E move:\n{output}"
            ));
        }
    }
    Ok(())
}

#[test]
fn gcode_serializer_formatting_probe() {
    serializer_formatting_probe().expect("H-GCODE-SERIALIZE formatter probe");
}

#[test]
fn proptest_serializer_extrusion_state() {
    let run = PropertyRun::new(SERIALIZE_PROPERTY_ID, SERIALIZE_TEST_NAME)
        .expect("H-GCODE-SERIALIZE registry context");
    let bounds = serialize_bounds(&run).expect("H-GCODE-SERIALIZE checked profile bounds");
    serializer_deterministic_witnesses().expect("H-GCODE-SERIALIZE deterministic witnesses");
    let strategy = serialize_case_strategy(bounds);
    run.check(&strategy, |case, _guard| {
        check_serializer_case(&case, SerializerMutation::None).map_err(TestCaseError::fail)
    })
    .expect("H-GCODE-SERIALIZE text and extrusion-state contract");
}

/// A witness case per mutation class, used by the controls.
fn serializer_mutation_witnesses() -> Vec<SerializeCase> {
    let move_command = |x_units: i64, e_units: Option<i64>| SerCommand::Move {
        x_units,
        y_units: 0,
        z_units: Some(1),
        e_units,
        f_units: Some(48_000),
        travel: false,
    };
    let retract = SerCommand::Retract {
        length_units: 16,
        speed_units: 640,
        mode: SerRetractMode::Gcode,
    };
    let unretract = SerCommand::Unretract {
        length_units: 16,
        speed_units: 640,
        mode: SerRetractMode::Gcode,
    };
    vec![
        SerializeCase {
            mode: SerializeMode::Relative,
            flavor: SerializeFlavor::Marlin,
            commands: vec![move_command(8, Some(32)), move_command(16, Some(64))],
        },
        SerializeCase {
            mode: SerializeMode::Absolute,
            flavor: SerializeFlavor::Marlin,
            commands: vec![
                move_command(8, Some(64)),
                retract.clone(),
                unretract.clone(),
                move_command(16, Some(96)),
                SerCommand::Raw("G92 E0".to_string()),
                retract,
                unretract,
                move_command(24, Some(4)),
            ],
        },
        SerializeCase {
            mode: SerializeMode::Relative,
            flavor: SerializeFlavor::Marlin,
            commands: vec![
                SerCommand::Move {
                    x_units: 8,
                    y_units: 0,
                    z_units: None,
                    e_units: None,
                    f_units: None,
                    travel: false,
                },
                move_command(16, Some(32)),
            ],
        },
    ]
}

/// Deterministic controls for `H-GCODE-SERIALIZE`: the independent renderer
/// accepts the real text and rejects one mutation per named defect class, the
/// formatter probe runs first, and the shrink/complicate witness probe keeps the
/// strategy honest.
#[test]
fn gcode_serializer_controls() {
    serializer_formatting_probe().expect("H-GCODE-SERIALIZE formatter probe");
    serializer_deterministic_witnesses().expect("H-GCODE-SERIALIZE deterministic witnesses");

    let witnesses = serializer_mutation_witnesses();
    let mutations = [
        SerializerMutation::ModeSwap,
        SerializerMutation::IgnoreG92,
        SerializerMutation::AccumulatorLost,
        SerializerMutation::UntrimmedFormatting,
        SerializerMutation::ReorderedHeader,
        SerializerMutation::MutatedDelta,
    ];
    for mutation in mutations {
        let mut rejected = 0_usize;
        for case in &witnesses {
            check_serializer_case(case, SerializerMutation::None)
                .expect("the documented renderer must accept every control witness");
            let commands = serializer_commands(case);
            let documented = render_serializer_oracle(case, &commands, SerializerMutation::None);
            let mutant = render_serializer_oracle(case, &commands, mutation);
            if documented == mutant {
                continue;
            }
            assert!(
                check_serializer_case(case, mutation).is_err(),
                "mutation {mutation:?} was not rejected on its witness"
            );
            rejected += 1;
        }
        assert!(
            rejected > 0,
            "mutation {mutation:?} produced no distinguishable witness; the control is vacuous"
        );
    }

    // Non-vacuous shrink/complicate probe over the same strategy and checker.
    let strategy = serialize_case_strategy(CONTROL_SERIALIZE_BOUNDS);
    shrink_complicate_witness(
        SERIALIZE_PROPERTY_ID,
        &strategy,
        |case| Some(format!("{:?}/{:?}", case.mode, case.flavor)),
        |case| check_serializer_case(case, SerializerMutation::None),
    );
}
