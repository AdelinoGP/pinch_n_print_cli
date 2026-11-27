//! Opt-in property tests for the public overhang speed-section and
//! interpolation functions (packet `proptest-openrscad_16_pbt-io-gcode`,
//! charter `H-OVERHANG-SPEED`). The generated property lives here; the
//! deterministic controls share the same checkers.
//!
//! Gated by the `property_testing` selection feature through the manifest
//! `[[test]]` entry, and skipped entirely by an ordinary build.

#![allow(missing_docs)]

use std::cell::Cell;

use overhang_classifier_default::{build_speed_sections, calculate_speed, OVERHANG_OVERLAP_LEVELS};
use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{
    Config as ProptestConfig, RngAlgorithm, RngSeed, TestCaseError, TestRunner,
};
use slicer_ir::ConfigView;
use slicer_sdk::test_prelude::ConfigViewBuilder;
use slicer_sdk::test_support::property_testing::PropertyRun;

const PROPERTY_ID: &str = "H-OVERHANG-SPEED";
const PROPERTY_TEST: &str = "proptest_overhang_speed_sections";

/// Independent copy of the six canonical overlap levels. `overhang_speed_controls`
/// pins it against `OVERHANG_OVERLAP_LEVELS`, so a mutation of the exported
/// constant is caught here instead of being inherited by the oracle.
const ORACLE_OVERLAP_LEVELS: [f32; 6] = [90.0, 75.0, 50.0, 25.0, 13.0, 0.0];

/// The declared value grid: millimetres and mm/s in eighths.
const EIGHTHS_PER_UNIT: f32 = 0.125;
/// Integer grid quanta per `max_path_width_mm` bound unit.
const QUANTA_PER_BOUND_UNIT: u64 = 8;
/// Guard against a mis-specified profile; the registry bound is 4 mm.
const MAX_PATH_QUANTA: u32 = 64 * 8;
/// Guard against a mis-specified profile; the registry bound is 200 mm/s.
const MAX_SPEED_UNITS: u32 = 20_000 * 8;
/// Upper bound on the integer `original_speed` domain.
const MAX_ORIGINAL_SPEED: u64 = 10_000;
/// Witnesses that must survive sample-budget truncation: below-first,
/// at-first, at-last, above-last, plus two strict in-interval samples.
const REQUIRED_SAMPLES: usize = 6;
/// Smallest path-width bound that can hold two distinct dyadic sections.
const MIN_PATH_QUANTA: u32 = 8;
/// Guard against a shrink tree that never settles.
const MAX_SHRINK_STEPS: usize = 20_000;

// -----------------------------------------------------------------------------
// Domain
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpeedMode {
    /// `build_speed_sections` on a positive path width.
    Sections,
    /// `build_speed_sections` at `path_width = 0`: six equal distances.
    ZeroWidth,
    /// `calculate_speed` with strictly increasing sections and nondecreasing
    /// speeds; the restricted monotonicity precondition is asserted.
    Interpolate,
    /// Strictly increasing sections with nonincreasing speeds; exact oracle
    /// only, no monotonicity claim.
    InterpolateDescending,
    /// A one-entry section list.
    Singleton,
    /// An empty section list.
    Empty,
    /// Duplicate, unsorted, negative or NaN sections: boundary observations.
    Boundary,
}

const SPEED_MODES: [SpeedMode; 7] = [
    SpeedMode::Sections,
    SpeedMode::ZeroWidth,
    SpeedMode::Interpolate,
    SpeedMode::InterpolateDescending,
    SpeedMode::Singleton,
    SpeedMode::Empty,
    SpeedMode::Boundary,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoundaryShape {
    /// Two entries share the first distance.
    DuplicateInterior,
    /// Two entries share the last distance.
    DuplicateLast,
    /// The last distance is below the first.
    UnsortedPair,
    /// The first distance is negative.
    NegativeFirst,
    /// The first distance is NaN.
    NanFirst,
}

const BOUNDARY_SHAPES: [BoundaryShape; 5] = [
    BoundaryShape::DuplicateInterior,
    BoundaryShape::DuplicateLast,
    BoundaryShape::UnsortedPair,
    BoundaryShape::NegativeFirst,
    BoundaryShape::NanFirst,
];

impl SpeedMode {
    fn label(self) -> &'static str {
        match self {
            SpeedMode::Sections => "sections",
            SpeedMode::ZeroWidth => "zero-width",
            SpeedMode::Interpolate => "interpolate",
            SpeedMode::InterpolateDescending => "interpolate-descending",
            SpeedMode::Singleton => "singleton",
            SpeedMode::Empty => "empty",
            SpeedMode::Boundary => "boundary",
        }
    }
}

impl BoundaryShape {
    fn label(self) -> &'static str {
        match self {
            BoundaryShape::DuplicateInterior => "duplicate-interior",
            BoundaryShape::DuplicateLast => "duplicate-last",
            BoundaryShape::UnsortedPair => "unsorted-pair",
            BoundaryShape::NegativeFirst => "negative-first",
            BoundaryShape::NanFirst => "nan-first",
        }
    }
}

/// Registry-profile bounds narrowed onto the exact dyadic grids.
#[derive(Clone, Copy, Debug)]
struct Limits {
    max_path_width: f32,
    max_speed: f32,
    path_quanta: u32,
    speed_units: u32,
    max_original_speed: u32,
    max_samples: usize,
}

fn units_to_speed(units: u32) -> f32 {
    units as f32 * EIGHTHS_PER_UNIT
}

fn units_to_distance(quanta: u32) -> f32 {
    quanta as f32 * EIGHTHS_PER_UNIT
}

fn limits_from_run(run: &PropertyRun) -> Limits {
    let path_width_raw = run
        .bound("max_path_width_mm")
        .expect("max_path_width_mm bound");
    let speed_raw = run.bound("max_speed_mm_s").expect("max_speed_mm_s bound");
    let samples_raw = run
        .bound("max_interpolation_samples")
        .expect("max_interpolation_samples bound");

    let path_quanta = path_width_raw
        .checked_mul(QUANTA_PER_BOUND_UNIT)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| (MIN_PATH_QUANTA..=MAX_PATH_QUANTA).contains(value))
        .unwrap_or_else(|| {
            panic!(
                "H-OVERHANG-SPEED profile: max_path_width_mm must be at least \
                 {MIN_PATH_QUANTA}/8 mm and at most {MAX_PATH_QUANTA}/8 mm, got {path_width_raw}"
            )
        });
    let speed_units = speed_raw
        .checked_mul(QUANTA_PER_BOUND_UNIT)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| (1..=MAX_SPEED_UNITS).contains(value))
        .unwrap_or_else(|| {
            panic!(
                "H-OVERHANG-SPEED profile: max_speed_mm_s must be positive and at most \
                 {MAX_SPEED_UNITS}/8 mm/s, got {speed_raw}"
            )
        });
    let max_original_speed = u32::try_from(speed_raw.min(MAX_ORIGINAL_SPEED))
        .ok()
        .filter(|value| *value >= 1)
        .unwrap_or_else(|| {
            panic!(
                "H-OVERHANG-SPEED profile: max_speed_mm_s must allow an integer \
                 original_speed in 1..={MAX_ORIGINAL_SPEED}, got {speed_raw}"
            )
        });
    let max_samples = usize::try_from(samples_raw)
        .ok()
        .filter(|value| *value >= REQUIRED_SAMPLES)
        .unwrap_or_else(|| {
            panic!(
                "H-OVERHANG-SPEED profile: max_interpolation_samples must be at least \
                 {REQUIRED_SAMPLES}, got {samples_raw}"
            )
        });

    Limits {
        max_path_width: units_to_distance(path_quanta),
        max_speed: units_to_speed(speed_units),
        path_quanta,
        speed_units,
        max_original_speed,
        max_samples,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SpeedCase {
    mode: SpeedMode,
    ref_speed: f32,
    overhang_speeds: [f32; 4],
    bridge_speed: f32,
    slowdown: bool,
    path_width: f32,
    sections: Vec<(f32, f32)>,
    samples: Vec<f32>,
    original_speed: f32,
    boundary: Option<BoundaryShape>,
}

#[derive(Debug)]
struct SpeedPick {
    ref_units: u32,
    overhang_units: [u32; 4],
    bridge_units: u32,
    slowdown: bool,
    sentinel: bool,
}

#[derive(Clone, Copy, Debug)]
struct Sketch {
    section_count: u32,
    start_quanta: u32,
    gap_quanta: u32,
    fraction_a: u32,
    fraction_b: u32,
    speed_seed: u32,
    boundary_shape: BoundaryShape,
}

// -----------------------------------------------------------------------------
// Strategy
// -----------------------------------------------------------------------------

fn speed_case_strategy(limits: &Limits) -> BoxedStrategy<SpeedCase> {
    let modes = proptest::sample::select(SPEED_MODES.to_vec()).no_shrink();
    let path_quanta = 1..=limits.path_quanta;
    let original_units = 1..=limits.max_original_speed;
    let speed_units = 0..=limits.speed_units;
    let ref_units = 1..=limits.speed_units;
    let pick = (
        ref_units,
        speed_units.clone(),
        speed_units.clone(),
        speed_units.clone(),
        speed_units.clone(),
        speed_units,
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(ref_units, first, second, third, fourth, bridge_units, slowdown, sentinel)| {
                SpeedPick {
                    ref_units,
                    overhang_units: [first, second, third, fourth],
                    bridge_units,
                    slowdown,
                    sentinel,
                }
            },
        );
    let sketch = (
        0u32..=2,
        0u32..=limits.path_quanta,
        0u32..=3,
        0u32..=2,
        0u32..=2,
        0u32..=limits.speed_units,
        proptest::sample::select(BOUNDARY_SHAPES.to_vec()).no_shrink(),
    )
        .prop_map(
            |(
                section_count,
                start_quanta,
                gap_quanta,
                fraction_a,
                fraction_b,
                speed_seed,
                boundary_shape,
            )| Sketch {
                section_count,
                start_quanta,
                gap_quanta,
                fraction_a,
                fraction_b,
                speed_seed,
                boundary_shape,
            },
        );
    let limits = *limits;
    (modes, path_quanta, original_units, pick, sketch)
        .prop_map(
            move |(mode, path_width_quanta, original_units, pick, sketch)| {
                build_case(
                    mode,
                    path_width_quanta,
                    original_units,
                    &pick,
                    &sketch,
                    &limits,
                )
            },
        )
        .boxed()
}

fn build_case(
    mode: SpeedMode,
    path_width_quanta: u32,
    original_units: u32,
    pick: &SpeedPick,
    sketch: &Sketch,
    limits: &Limits,
) -> SpeedCase {
    let ref_speed = units_to_speed(pick.ref_units);
    let mut overhang_units = pick.overhang_units;
    if pick.sentinel {
        // Force the `< 0.5` sentinel and its exact strict boundary (0.5) into
        // the case: both are vanishingly rare in a uniform 1/8-unit grid up to
        // `max_speed_mm_s`, so sampling alone would not reliably exercise the
        // substitution rule.
        overhang_units[0] = pick.overhang_units[0] % 4; // 0, 1/8, 1/4, 3/8
        overhang_units[1] = 4; // exactly 0.5, which must NOT be substituted
    }
    let overhang_speeds = [
        units_to_speed(overhang_units[0]),
        units_to_speed(overhang_units[1]),
        units_to_speed(overhang_units[2]),
        units_to_speed(overhang_units[3]),
    ];
    let bridge_speed = units_to_speed(pick.bridge_units);
    let path_width = if mode == SpeedMode::ZeroWidth {
        0.0
    } else {
        units_to_distance(path_width_quanta)
    };
    debug_assert!(path_width <= limits.max_path_width);
    debug_assert!(ref_speed <= limits.max_speed);
    let original_speed = original_units as f32;

    let (sections, samples, boundary) = match mode {
        SpeedMode::Sections | SpeedMode::ZeroWidth => (Vec::new(), Vec::new(), None),
        SpeedMode::Empty => (
            Vec::new(),
            vec![-1.0, 0.0, 0.5, units_to_distance(limits.path_quanta)],
            None,
        ),
        SpeedMode::Singleton => {
            let distance = units_to_distance(path_width_quanta);
            let speed = units_to_speed(sketch.speed_seed);
            (
                vec![(distance, speed)],
                vec![
                    distance - EIGHTHS_PER_UNIT,
                    distance,
                    distance + EIGHTHS_PER_UNIT,
                ],
                None,
            )
        }
        SpeedMode::Interpolate | SpeedMode::InterpolateDescending => {
            let sections =
                interpolation_sections(sketch, limits, mode == SpeedMode::InterpolateDescending);
            let samples = interpolation_samples(&sections, sketch, limits);
            (sections, samples, None)
        }
        SpeedMode::Boundary => {
            let (sections, samples) = boundary_sections(
                sketch.boundary_shape,
                path_width_quanta,
                sketch.speed_seed,
                limits,
            );
            (sections, samples, Some(sketch.boundary_shape))
        }
    };

    SpeedCase {
        mode,
        ref_speed,
        overhang_speeds,
        bridge_speed,
        slowdown: pick.slowdown,
        path_width,
        sections,
        samples,
        original_speed,
        boundary,
    }
}

/// Strictly increasing dyadic section distances with sorted dyadic speeds.
fn interpolation_sections(sketch: &Sketch, limits: &Limits, descending: bool) -> Vec<(f32, f32)> {
    let max_quanta = u64::from(limits.path_quanta);
    let start = u64::from(sketch.start_quanta) % (max_quanta / 2 + 1);
    let gap = 1 + u64::from(sketch.gap_quanta) % 4;
    let max_steps = (max_quanta - start) / gap;
    let wanted = 2 + usize::try_from(sketch.section_count).expect("section count fits usize");
    let count = wanted
        .min(usize::try_from(max_steps).expect("step count fits usize") + 1)
        .max(2);
    let mut speed_units: Vec<u32> = (0..count)
        .map(|index| {
            let mixed = u64::from(sketch.speed_seed)
                .wrapping_mul(37)
                .wrapping_add(index as u64 * 53)
                .wrapping_add(11);
            u32::try_from(mixed % (u64::from(limits.speed_units) + 1)).expect("bounded speed")
        })
        .collect();
    speed_units.sort_unstable();
    if descending {
        speed_units.reverse();
    }
    (0..count)
        .map(|index| {
            let quanta = start + index as u64 * gap;
            (
                units_to_distance(u32::try_from(quanta).expect("bounded distance")),
                units_to_speed(speed_units[index]),
            )
        })
        .collect()
}

fn interpolation_samples(sections: &[(f32, f32)], sketch: &Sketch, limits: &Limits) -> Vec<f32> {
    let first = sections[0].0;
    let last = sections[sections.len() - 1].0;
    let (d0, _) = sections[0];
    let (d1, _) = sections[1];
    // Both fractions are exact eighths and lie strictly inside the interval.
    let fraction_a = units_to_distance(1 + sketch.fraction_a); // 1/8 ..= 3/8
    let fraction_b = units_to_distance(5 + sketch.fraction_b); // 5/8 ..= 7/8
    let in_a = d0 + fraction_a * (d1 - d0);
    let in_b = d0 + fraction_b * (d1 - d0);
    let mut samples = vec![
        first - EIGHTHS_PER_UNIT,
        first,
        last,
        last + EIGHTHS_PER_UNIT,
        in_a,
        in_b,
    ];
    for (distance, _) in sections
        .iter()
        .skip(1)
        .take(sections.len().saturating_sub(2))
    {
        samples.push(*distance);
    }
    samples.truncate(limits.max_samples);
    samples
}

fn boundary_sections(
    shape: BoundaryShape,
    path_width_quanta: u32,
    speed_seed: u32,
    limits: &Limits,
) -> (Vec<(f32, f32)>, Vec<f32>) {
    let base = units_to_distance(path_width_quanta);
    let speed = |offset: u32| {
        units_to_speed(speed_seed.wrapping_add(offset.wrapping_mul(97)) % (limits.speed_units + 1))
    };
    let (s0, s1, s2) = (speed(0), speed(1), speed(2));
    match shape {
        BoundaryShape::DuplicateInterior => (
            vec![(base, s0), (base, s1), (base + 0.25, s2)],
            vec![
                base - EIGHTHS_PER_UNIT,
                base,
                base + 0.125,
                base + 0.25,
                base + 0.375,
            ],
        ),
        BoundaryShape::DuplicateLast => (
            vec![(base, s0), (base + 0.25, s1), (base + 0.25, s2)],
            vec![
                base - EIGHTHS_PER_UNIT,
                base,
                base + 0.125,
                base + 0.25,
                base + 0.375,
            ],
        ),
        BoundaryShape::UnsortedPair => (
            vec![(base + 1.0, s0), (base, s1)],
            vec![
                base - EIGHTHS_PER_UNIT,
                base,
                base + 0.5,
                base + 1.0,
                base + 1.125,
            ],
        ),
        BoundaryShape::NegativeFirst => (
            vec![(-0.5, s0), (base, s1)],
            vec![-0.75, -0.5, -0.25, 0.0, base, base + EIGHTHS_PER_UNIT],
        ),
        BoundaryShape::NanFirst => (
            vec![(f32::NAN, s0), (base, s1)],
            vec![0.0, base - EIGHTHS_PER_UNIT, base, base + EIGHTHS_PER_UNIT],
        ),
    }
}

// -----------------------------------------------------------------------------
// Independent oracles
// -----------------------------------------------------------------------------

fn config_for(case: &SpeedCase) -> ConfigView {
    ConfigViewBuilder::new()
        .bool("enable_overhang_speed", true)
        .bool("slowdown_for_curled_perimeters", case.slowdown)
        .float("outer_wall_line_width", 0.0)
        .float("outer_wall_speed", f64::from(case.ref_speed))
        .float("inner_wall_speed", f64::from(case.ref_speed))
        .float("thin_wall_speed", f64::from(case.ref_speed))
        .float("overhang_1_4_speed", f64::from(case.overhang_speeds[0]))
        .float("overhang_2_4_speed", f64::from(case.overhang_speeds[1]))
        .float("overhang_3_4_speed", f64::from(case.overhang_speeds[2]))
        .float("overhang_4_4_speed", f64::from(case.overhang_speeds[3]))
        .float("bridge_speed", f64::from(case.bridge_speed))
        .float("line_width", f64::from(case.path_width))
        .build()
}

/// The six expected sections, built from the documented rules and sorted by an
/// insertion sort over the declared key order (ascending distance, ties by
/// descending speed). Equal-distance runs are flattened to the run's first
/// (highest) speed, and every entry is retained.
fn expected_sections(case: &SpeedCase) -> Vec<(f32, f32)> {
    let substitute = |configured: f32| {
        if configured < 0.5 {
            case.ref_speed
        } else {
            configured
        }
    };
    let sixth = if case.slowdown {
        substitute(case.overhang_speeds[3])
    } else {
        case.bridge_speed
    };
    let speeds = [
        case.ref_speed,
        substitute(case.overhang_speeds[0]),
        substitute(case.overhang_speeds[1]),
        substitute(case.overhang_speeds[2]),
        substitute(case.overhang_speeds[3]),
        sixth,
    ];
    let mut entries: Vec<(f32, f32)> = ORACLE_OVERLAP_LEVELS
        .iter()
        .zip(speeds)
        .map(|(level, speed)| (case.path_width * (1.0 - level / 100.0), speed))
        .collect();

    for index in 1..entries.len() {
        let mut position = index;
        while position > 0 && section_precedes(entries[position], entries[position - 1]) {
            entries.swap(position, position - 1);
            position -= 1;
        }
    }

    let mut run_start = 0;
    for index in 1..entries.len() {
        if entries[index].0 == entries[run_start].0 {
            entries[index].1 = entries[run_start].1;
        } else {
            run_start = index;
        }
    }
    entries
}

fn section_precedes(left: (f32, f32), right: (f32, f32)) -> bool {
    if left.0 < right.0 {
        true
    } else if left.0 > right.0 {
        false
    } else {
        left.1 > right.1
    }
}

/// `min(round(branch_value), original_speed)` for the documented branches.
/// Empty sections return `original_speed` (a PnP boundary behaviour, not a
/// canonical contract).
fn expected_speed(distance: f32, sections: &[(f32, f32)], original_speed: f32) -> f32 {
    if sections.is_empty() {
        return original_speed;
    }
    let first_distance = sections[0].0;
    let (last_distance, last_speed) = sections[sections.len() - 1];
    if distance <= first_distance {
        return original_speed;
    }
    if distance >= last_distance {
        return last_speed.round().min(original_speed);
    }
    let upper = sections
        .iter()
        .position(|(section_distance, _)| *section_distance >= distance)
        .expect("distance is bracketed by the section list");
    let (d0, s0) = sections[upper - 1];
    let (d1, s1) = sections[upper];
    // Exact on the declared dyadic grid, so this form and the production
    // `(1 - t) * s0 + t * s1` both equal the real value bit for bit.
    let t = (distance - d0) / (d1 - d0);
    (s0 + t * (s1 - s0)).round().min(original_speed)
}

fn check_sections(actual: &[(f32, f32)], expected: &[(f32, f32)]) -> Result<(), String> {
    if actual.len() != ORACLE_OVERLAP_LEVELS.len() {
        return Err(format!(
            "expected {} entries, got {}: {actual:?}",
            ORACLE_OVERLAP_LEVELS.len(),
            actual.len()
        ));
    }
    if expected.len() != actual.len() {
        return Err(format!(
            "oracle produced {} entries, expected {}",
            expected.len(),
            actual.len()
        ));
    }
    for (index, (actual_entry, expected_entry)) in actual.iter().zip(expected).enumerate() {
        if actual_entry.0.to_bits() != expected_entry.0.to_bits()
            || actual_entry.1.to_bits() != expected_entry.1.to_bits()
        {
            return Err(format!(
                "entry {index} is ({}, {}), expected ({}, {}); actual {actual:?}, expected {expected:?}",
                actual_entry.0, actual_entry.1, expected_entry.0, expected_entry.1
            ));
        }
    }
    Ok(())
}

fn check_speed_value(
    distance: f32,
    sections: &[(f32, f32)],
    original_speed: f32,
    expected: f32,
) -> Result<(), String> {
    let actual = calculate_speed(distance, sections, original_speed);
    if actual.to_bits() != expected.to_bits() {
        return Err(format!(
            "calculate_speed({distance}, {sections:?}, {original_speed}) is {actual} \
             ({:#010x}), expected {expected} ({:#010x})",
            actual.to_bits(),
            expected.to_bits()
        ));
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Property
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
struct Stats {
    sections_checked: usize,
    zero_width_witnesses: usize,
    interpolation_samples: usize,
    empty_samples: usize,
    singleton_samples: usize,
    monotone_comparisons: usize,
    boundary_observations: usize,
    below_sentinel_speeds: usize,
    exact_half_speeds: usize,
    slowdown_true: usize,
    slowdown_false: usize,
}

fn check_case(case: &SpeedCase) -> Result<Stats, String> {
    let mut stats = Stats::default();
    match case.mode {
        SpeedMode::Sections | SpeedMode::ZeroWidth => {
            let config = config_for(case);
            let actual = build_speed_sections(case.ref_speed, case.path_width, &config)
                .map_err(|error| format!("build_speed_sections failed: {error:?}"))?;
            let expected = expected_sections(case);
            check_sections(&actual, &expected)?;
            stats.sections_checked += 1;
            for configured in case.overhang_speeds {
                if configured < 0.5 {
                    stats.below_sentinel_speeds += 1;
                }
                if configured.to_bits() == 0.5f32.to_bits() {
                    stats.exact_half_speeds += 1;
                }
            }
            if case.slowdown {
                stats.slowdown_true += 1;
            } else {
                stats.slowdown_false += 1;
            }
            if case.mode == SpeedMode::ZeroWidth {
                if actual.iter().any(|(distance, _)| *distance != 0.0) {
                    return Err(format!(
                        "path_width = 0 must yield six zero distances, got {actual:?}"
                    ));
                }
                let flattened = actual[0].1;
                if actual.iter().any(|(_, speed)| *speed != flattened) {
                    return Err(format!(
                        "path_width = 0 must flatten every entry to the first/highest \
                         speed {flattened}, got {actual:?}"
                    ));
                }
                stats.zero_width_witnesses += 1;
            }
        }
        SpeedMode::Empty => {
            for sample in &case.samples {
                check_speed_value(
                    *sample,
                    &case.sections,
                    case.original_speed,
                    case.original_speed,
                )?;
                stats.empty_samples += 1;
            }
        }
        SpeedMode::Singleton => {
            for sample in &case.samples {
                let expected = expected_speed(*sample, &case.sections, case.original_speed);
                check_speed_value(*sample, &case.sections, case.original_speed, expected)?;
                stats.singleton_samples += 1;
            }
        }
        SpeedMode::Interpolate | SpeedMode::InterpolateDescending => {
            for sample in &case.samples {
                let expected = expected_speed(*sample, &case.sections, case.original_speed);
                check_speed_value(*sample, &case.sections, case.original_speed, expected)?;
                stats.interpolation_samples += 1;
            }
            if case.mode == SpeedMode::Interpolate {
                let first = case.sections[0].0;
                let last = case.sections[case.sections.len() - 1].0;
                let mut points: Vec<f32> = case
                    .samples
                    .iter()
                    .copied()
                    .filter(|sample| *sample > first && *sample <= last)
                    .collect();
                points.sort_by(|left, right| left.total_cmp(right));
                points.dedup_by(|left, right| left.to_bits() == right.to_bits());
                if points.len() >= 2 {
                    let a = points[points.len() - 2];
                    let b = points[points.len() - 1];
                    let speed_a = calculate_speed(a, &case.sections, case.original_speed);
                    let speed_b = calculate_speed(b, &case.sections, case.original_speed);
                    if speed_a > speed_b {
                        return Err(format!(
                            "nondecreasing output violated for first_distance < {a} < {b} <= \
                             last_distance: {speed_a} > {speed_b} (sections {:?})",
                            case.sections
                        ));
                    }
                    stats.monotone_comparisons += 1;
                }
            }
        }
        SpeedMode::Boundary => {
            let shape = case
                .boundary
                .expect("the boundary mode always records its shape");
            match shape {
                BoundaryShape::DuplicateInterior => {
                    if case.sections.len() != 3 || case.sections[0].0 != case.sections[1].0 {
                        return Err(format!(
                            "duplicate-interior witness must retain three entries with two \
                             equal first distances, got {:?}",
                            case.sections
                        ));
                    }
                }
                BoundaryShape::DuplicateLast => {
                    if case.sections.len() != 3 || case.sections[1].0 != case.sections[2].0 {
                        return Err(format!(
                            "duplicate-last witness must retain three entries with two equal \
                             last distances, got {:?}",
                            case.sections
                        ));
                    }
                }
                BoundaryShape::UnsortedPair => {
                    if case.sections[0].0 <= case.sections[1].0 {
                        return Err(format!(
                            "unsorted-pair witness must not be sorted: {:?}",
                            case.sections
                        ));
                    }
                }
                BoundaryShape::NegativeFirst => {
                    if case.sections[0].0 >= 0.0 {
                        return Err(format!(
                            "negative-first witness must start below zero: {:?}",
                            case.sections
                        ));
                    }
                }
                BoundaryShape::NanFirst => {
                    if !case.sections[0].0.is_nan() {
                        return Err(format!(
                            "nan-first witness must start with NaN: {:?}",
                            case.sections
                        ));
                    }
                }
            }
            for sample in &case.samples {
                let expected = expected_speed(*sample, &case.sections, case.original_speed);
                check_speed_value(*sample, &case.sections, case.original_speed, expected)?;
            }
            stats.boundary_observations += 1;
        }
    }
    Ok(stats)
}

#[test]
fn proptest_overhang_speed_sections() {
    let run = PropertyRun::new(PROPERTY_ID, PROPERTY_TEST)
        .expect("H-OVERHANG-SPEED registry context (cargo xtask test --properties)");
    let limits = limits_from_run(&run);
    let strategy = speed_case_strategy(&limits);

    let mode_counts: [Cell<u64>; SPEED_MODES.len()] = Default::default();
    let sections_checked = Cell::new(0usize);
    let zero_width_witnesses = Cell::new(0usize);
    let interpolation_samples = Cell::new(0usize);
    let empty_samples = Cell::new(0usize);
    let singleton_samples = Cell::new(0usize);
    let monotone_comparisons = Cell::new(0usize);
    let boundary_observations = Cell::new(0usize);
    let below_sentinel_speeds = Cell::new(0usize);
    let exact_half_speeds = Cell::new(0usize);
    let slowdown_true = Cell::new(0usize);
    let slowdown_false = Cell::new(0usize);

    run.check(&strategy, |case, _guard| {
        let slot = SPEED_MODES
            .iter()
            .position(|mode| *mode == case.mode)
            .expect("the generated mode is one of SPEED_MODES");
        mode_counts[slot].set(mode_counts[slot].get() + 1);
        match check_case(&case) {
            Ok(stats) => {
                sections_checked.set(sections_checked.get() + stats.sections_checked);
                zero_width_witnesses.set(zero_width_witnesses.get() + stats.zero_width_witnesses);
                interpolation_samples
                    .set(interpolation_samples.get() + stats.interpolation_samples);
                empty_samples.set(empty_samples.get() + stats.empty_samples);
                singleton_samples.set(singleton_samples.get() + stats.singleton_samples);
                monotone_comparisons.set(monotone_comparisons.get() + stats.monotone_comparisons);
                boundary_observations
                    .set(boundary_observations.get() + stats.boundary_observations);
                below_sentinel_speeds
                    .set(below_sentinel_speeds.get() + stats.below_sentinel_speeds);
                exact_half_speeds.set(exact_half_speeds.get() + stats.exact_half_speeds);
                slowdown_true.set(slowdown_true.get() + stats.slowdown_true);
                slowdown_false.set(slowdown_false.get() + stats.slowdown_false);
            }
            Err(error) => {
                return Err(TestCaseError::fail(format!(
                    "{} case: {error}",
                    case.mode.label()
                )))
            }
        }
        Ok(())
    })
    .expect("H-OVERHANG-SPEED six-entry sections and piecewise interpolation");

    for (mode, counter) in SPEED_MODES.iter().zip(mode_counts.iter()) {
        assert!(
            counter.get() > 0,
            "the property never generated the {} mode; every mode must be exercised",
            mode.label()
        );
    }
    assert!(sections_checked.get() > 0, "no section list was checked");
    assert!(
        zero_width_witnesses.get() > 0,
        "no path_width = 0 duplicate-distance witness was generated"
    );
    assert!(
        interpolation_samples.get() > 0,
        "no interpolation sample was checked"
    );
    assert!(
        empty_samples.get() > 0,
        "no empty-section sample was checked"
    );
    assert!(
        singleton_samples.get() > 0,
        "no singleton-section sample was checked"
    );
    assert!(
        monotone_comparisons.get() > 0,
        "no restricted monotonicity comparison ran"
    );
    assert!(
        boundary_observations.get() > 0,
        "no boundary-shape observation ran"
    );
    assert!(
        below_sentinel_speeds.get() > 0,
        "no configured overhang speed below the 0.5 sentinel was generated"
    );
    assert!(
        exact_half_speeds.get() > 0,
        "no configured overhang speed at the exact 0.5 strict boundary was generated"
    );
    assert!(
        slowdown_true.get() > 0,
        "no section case used slowdown_for_curled_perimeters = true"
    );
    assert!(
        slowdown_false.get() > 0,
        "no section case used slowdown_for_curled_perimeters = false"
    );
    println!(
        "H-OVERHANG-SPEED: sections={} zero_width={} interpolation_samples={} empty={} \
         singleton={} monotone={} boundary={} below_sentinel={} exact_half={} \
         slowdown_true={} slowdown_false={} modes={:?}",
        sections_checked.get(),
        zero_width_witnesses.get(),
        interpolation_samples.get(),
        empty_samples.get(),
        singleton_samples.get(),
        monotone_comparisons.get(),
        boundary_observations.get(),
        below_sentinel_speeds.get(),
        exact_half_speeds.get(),
        slowdown_true.get(),
        slowdown_false.get(),
        SPEED_MODES.map(SpeedMode::label),
    );
}

// -----------------------------------------------------------------------------
// Deterministic controls
// -----------------------------------------------------------------------------

fn control_limits() -> Limits {
    Limits {
        max_path_width: units_to_distance(32),
        max_speed: units_to_speed(1600),
        path_quanta: 32,
        speed_units: 1600,
        max_original_speed: 200,
        max_samples: 8,
    }
}

fn concrete_case(
    mode: SpeedMode,
    ref_speed: f32,
    overhang_speeds: [f32; 4],
    bridge_speed: f32,
    slowdown: bool,
    path_width: f32,
) -> SpeedCase {
    SpeedCase {
        mode,
        ref_speed,
        overhang_speeds,
        bridge_speed,
        slowdown,
        path_width,
        sections: Vec::new(),
        samples: Vec::new(),
        original_speed: 40.0,
        boundary: None,
    }
}

#[test]
fn overhang_speed_controls() {
    pinned_overlap_levels();
    section_controls();
    zero_width_controls();
    defect_boundary_witness();
    interpolation_controls();
    monotonicity_counterexample();
    boundary_observations();
    shrink_witness_probe(&control_limits());
    println!("H-OVERHANG-SPEED controls: all named mutations rejected");
}

fn pinned_overlap_levels() {
    assert_eq!(
        OVERHANG_OVERLAP_LEVELS, ORACLE_OVERLAP_LEVELS,
        "the six overlap levels are the pinned canonical list"
    );
}

fn section_controls() {
    let case = concrete_case(
        SpeedMode::Sections,
        60.0,
        [30.0, 40.0, 50.0, 44.0],
        25.0,
        false,
        1.0,
    );
    let expected = expected_sections(&case);
    assert_eq!(
        expected,
        vec![
            (1.0 - 90.0 / 100.0, 60.0),
            (1.0 - 75.0 / 100.0, 30.0),
            (1.0 - 50.0 / 100.0, 40.0),
            (1.0 - 25.0 / 100.0, 50.0),
            (1.0 - 13.0 / 100.0, 44.0),
            (1.0 - 0.0 / 100.0, 25.0),
        ],
        "ascending distance, descending tie speeds, sixth slot = bridge_speed"
    );
    let actual = build_speed_sections(60.0, 1.0, &config_for(&case)).expect("sections");
    check_sections(&actual, &expected).expect("the correct expected sections are accepted");

    // Missing `< 0.5` sentinel: the configured 0.25 must resolve to ref_speed,
    // not to the literal configured value.
    let mut sentinel_case = case.clone();
    sentinel_case.overhang_speeds[0] = 0.25;
    let sentinel_expected = expected_sections(&sentinel_case);
    assert_eq!(
        sentinel_expected[1].1, 60.0,
        "a configured speed below 0.5 is replaced by ref_speed"
    );
    let mut missing_sentinel = sentinel_expected.clone();
    missing_sentinel[1].1 = 0.25;
    assert!(
        check_sections(&missing_sentinel, &sentinel_expected).is_err(),
        "control must reject an unsubstituted `< 0.5` configured speed"
    );

    // Wrong sixth slot: without slowdown the sixth entry is bridge_speed, even
    // though overhang_4_4_speed differs.
    let mut wrong_sixth = expected.clone();
    wrong_sixth[5].1 = 44.0;
    assert!(
        check_sections(&wrong_sixth, &expected).is_err(),
        "control must reject overhang_4_4_speed in the sixth slot when slowdown is off"
    );
    let slowdown_case = concrete_case(
        SpeedMode::Sections,
        60.0,
        [30.0, 40.0, 50.0, 44.0],
        25.0,
        true,
        1.0,
    );
    let slowdown_expected = expected_sections(&slowdown_case);
    assert_eq!(
        slowdown_expected[5].1, 44.0,
        "with slowdown the sixth slot is the overhang_4_4_speed rule"
    );
    assert!(
        check_sections(&expected, &slowdown_expected).is_err(),
        "control must reject the bridge sixth slot when slowdown is on"
    );
}

fn zero_width_controls() {
    let case = concrete_case(
        SpeedMode::ZeroWidth,
        60.0,
        [30.0, 40.0, 50.0, 44.0],
        25.0,
        false,
        0.0,
    );
    let expected = expected_sections(&case);
    assert_eq!(expected.len(), 6, "all six entries are retained");
    assert!(
        expected
            .iter()
            .all(|(distance, speed)| *distance == 0.0 && *speed == 60.0),
        "six zero distances flatten to the first/highest speed: {expected:?}"
    );
    let actual = build_speed_sections(60.0, 0.0, &config_for(&case)).expect("sections");
    check_sections(&actual, &expected).expect("the zero-width witness is accepted");

    // A dropped duplicate entry (dedup instead of flatten) must be rejected.
    let dropped = expected[..expected.len() - 1].to_vec();
    assert!(
        check_sections(&dropped, &expected).is_err(),
        "control must reject a deduplicated equal-distance run"
    );

    // Ascending tie order would keep the lowest speed first and flatten the run
    // to it; the rule is descending speed -> the run's first speed is the max.
    let ascending_ties = vec![(0.0f32, 25.0f32); 6];
    assert!(
        check_sections(&ascending_ties, &expected).is_err(),
        "control must reject ties sorted by ascending speed"
    );

    // Unequal tie speeds (no flattening at all) must be rejected.
    let mut unequal_ties = expected.clone();
    for (entry, speed) in unequal_ties
        .iter_mut()
        .zip([60.0, 30.0, 40.0, 50.0, 44.0, 25.0])
    {
        entry.1 = speed;
    }
    assert!(
        check_sections(&unequal_ties, &expected).is_err(),
        "control must reject an equal-distance run whose entries keep unequal speeds"
    );
}

fn defect_boundary_witness() {
    // The confirmed H-OVERHANG-SPEED defect: the `>= last` branch returned the
    // raw last-section speed, skipping both `round` and the `original_speed`
    // cap. Root cause: `calculate_speed`'s last-boundary branch read
    // `sections[len - 1].1` instead of `sections[len - 1].1.round().min(original_speed)`.
    let sections = [(0.0f32, 1.0f32), (1.0f32, 1.75f32)];
    assert_eq!(
        calculate_speed(0.875, &sections, 2.0),
        2.0,
        "the interior sample rounds 1.65625 up to 2.0 and the cap leaves it at 2.0"
    );
    assert_eq!(
        calculate_speed(1.0, &sections, 2.0),
        2.0,
        "the last boundary must round 1.75 up to 2.0 (formerly returned 1.75)"
    );
    assert_eq!(calculate_speed(2.0, &sections, 2.0), 2.0);

    // Missing last-boundary round-and-cap: the unrounded 1.75 expectation is
    // rejected, and a last-section speed above original_speed is capped.
    assert!(
        check_speed_value(1.0, &sections, 2.0, 1.75).is_err(),
        "control must reject the unrounded/uncapped last-boundary value"
    );
    let fast = [(0.0f32, 10.0f32), (1.0f32, 20.0f32)];
    assert_eq!(calculate_speed(1.5, &fast, 15.0), 15.0);
    assert!(
        check_speed_value(1.5, &fast, 15.0, 20.0).is_err(),
        "control must reject the last-boundary value before the original_speed cap"
    );
}

fn interpolation_controls() {
    // Empty sections: PnP boundary behaviour, not a canonical contract.
    let empty: Vec<(f32, f32)> = Vec::new();
    assert_eq!(calculate_speed(0.5, &empty, 40.0), 40.0);

    // Singleton: the `<= first` check precedes `>= last`.
    let singleton = [(1.0f32, 1.75f32)];
    assert_eq!(calculate_speed(0.5, &singleton, 2.0), 2.0);
    assert_eq!(calculate_speed(1.0, &singleton, 2.0), 2.0);
    assert_eq!(calculate_speed(1.5, &singleton, 2.0), 2.0);
    assert_eq!(calculate_speed(1.5, &singleton, 1.0), 1.0);

    // Below the first section returns original_speed.
    let rising = [(0.5f32, 10.0f32), (1.0f32, 20.0f32)];
    assert_eq!(calculate_speed(0.0, &rising, 30.0), 30.0);
    assert!(
        check_speed_value(0.0, &rising, 30.0, 10.0).is_err(),
        "control must reject the first-section speed below the first distance"
    );

    // Last-boundary round is half away from zero.
    let half = [(0.0f32, 10.0f32), (1.0f32, 21.5f32)];
    assert_eq!(calculate_speed(1.0, &half, 40.0), 22.0);

    // Wrong interval selection: at 2.0 the correct bracket is (1, 3), not
    // (0, 1) with a clamped t.
    let three = [(0.0f32, 10.0f32), (1.0f32, 20.0f32), (3.0f32, 30.0f32)];
    assert_eq!(calculate_speed(2.0, &three, 40.0), 25.0);
    assert!(
        check_speed_value(2.0, &three, 40.0, 20.0).is_err(),
        "control must reject the wrong bracketing interval"
    );

    // Missing round in the interpolation branch: 15.75 must become 16.
    let fractional = [(0.0f32, 10.0f32), (2.0f32, 21.5f32)];
    assert_eq!(calculate_speed(1.0, &fractional, 40.0), 16.0);
    assert!(
        check_speed_value(1.0, &fractional, 40.0, 15.75).is_err(),
        "control must reject an unrounded interpolated value"
    );

    // `original_speed` below, at and above the interpolation branch value.
    let ramp = [(0.0f32, 10.0f32), (2.0f32, 30.0f32)];
    assert_eq!(calculate_speed(1.5, &ramp, 20.0), 20.0);
    assert_eq!(calculate_speed(1.5, &ramp, 25.0), 25.0);
    assert_eq!(calculate_speed(1.5, &ramp, 40.0), 25.0);

    // `original_speed` below, at and above the last section speed.
    let last_ramp = [(0.0f32, 10.0f32), (1.0f32, 20.0f32)];
    assert_eq!(calculate_speed(2.0, &last_ramp, 15.0), 15.0);
    assert_eq!(calculate_speed(2.0, &last_ramp, 20.0), 20.0);
    assert_eq!(calculate_speed(2.0, &last_ramp, 25.0), 20.0);
}

fn monotonicity_counterexample() {
    // Global monotonicity does not hold because the `<= first` branch returns
    // `original_speed`: with [(0, 10), (1, 20)] and original_speed 30, the
    // output drops from 30 at the first distance to 15 inside the interval.
    // The property therefore asserts nondecreasing output only for
    // `first_distance < a < b <= last_distance`.
    let sections = [(0.0f32, 10.0f32), (1.0f32, 20.0f32)];
    let at_first = calculate_speed(0.0, &sections, 30.0);
    let inside = calculate_speed(0.5, &sections, 30.0);
    assert_eq!(at_first, 30.0);
    assert_eq!(inside, 15.0);
    assert!(
        at_first > inside,
        "recorded `<= first` counterexample: {at_first} > {inside}"
    );
    let later = calculate_speed(0.75, &sections, 30.0);
    assert!(
        inside <= later,
        "{inside} <= {later} inside the restricted window"
    );
}

fn boundary_observations() {
    let limits = control_limits();
    for shape in BOUNDARY_SHAPES {
        let (sections, samples) = boundary_sections(shape, 32, 7, &limits);
        let case = SpeedCase {
            mode: SpeedMode::Boundary,
            ref_speed: 60.0,
            overhang_speeds: [30.0, 40.0, 50.0, 44.0],
            bridge_speed: 25.0,
            slowdown: false,
            path_width: 4.0,
            sections,
            samples,
            original_speed: 40.0,
            boundary: Some(shape),
        };
        check_case(&case).unwrap_or_else(|error| {
            panic!("{} boundary observation failed: {error}", shape.label())
        });
    }
    println!(
        "H-OVERHANG-SPEED boundary observations: {:?} (classified, not a contract claim)",
        BOUNDARY_SHAPES.map(BoundaryShape::label)
    );
}

/// Shrink/complicate witness probe. The semantic mode selector is
/// `prop::sample::select(...).no_shrink()`, so a mode must survive every
/// simplify/complicate transition while the numeric payload keeps shrinking.
fn shrink_witness_probe(limits: &Limits) {
    let strategy = speed_case_strategy(limits);
    let runner_config = |seed: u64| ProptestConfig {
        cases: 1,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    let mut observed_simplify = 0usize;
    let mut observed_restores = 0usize;
    let mut round_trip = false;

    for attempt in 0..8u64 {
        let seed = 0x5eed_c0de_u64.wrapping_add(attempt);

        // Phase 1/2: accept every simplification and revalidate each state,
        // then undo whatever the tree can restore.
        let mut runner = TestRunner::new(runner_config(seed));
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("the composed strategy always produces a case");
        let initial = tree.current();
        check_case(&initial).expect("the initial case is valid");
        let mode = initial.mode;

        let mut simplified = 0usize;
        while tree.simplify() {
            simplified += 1;
            assert!(
                simplified < MAX_SHRINK_STEPS,
                "shrink probe: simplify never exhausted"
            );
            let case = tree.current();
            assert_eq!(
                case.mode, mode,
                "the semantic mode must survive simplify (no_shrink selector)"
            );
            check_case(&case)
                .unwrap_or_else(|error| panic!("shrunk {} case: {error}", case.mode.label()));
        }
        observed_simplify += simplified;
        let mut restored = 0usize;
        while tree.complicate() {
            restored += 1;
            assert!(
                restored < MAX_SHRINK_STEPS,
                "shrink probe: complicate never exhausted"
            );
            let case = tree.current();
            assert_eq!(
                case.mode, mode,
                "the semantic mode must survive complicate (no_shrink selector)"
            );
            check_case(&case)
                .unwrap_or_else(|error| panic!("restored {} case: {error}", case.mode.label()));
        }
        observed_restores += restored;

        // Phase 3: a successful simplify must be followed by a complicate
        // transition that still revalidates. This is the real complicate
        // witness; a fully exhausted tree legitimately restores nothing, so it
        // cannot supply one. Proptest's `complicate` is a heuristic rather
        // than an exact inverse, so the restored value is revalidated, not
        // compared for equality.
        let mut round_trip_runner = TestRunner::new(runner_config(seed));
        let mut round_trip_tree = strategy
            .new_tree(&mut round_trip_runner)
            .expect("the composed strategy always produces a case");
        let before = round_trip_tree.current();
        if round_trip_tree.simplify() {
            let simpler = round_trip_tree.current();
            assert_eq!(
                simpler.mode, before.mode,
                "the semantic mode must survive the simplify half of the round trip"
            );
            check_case(&simpler).expect("the simplified case is valid");
            assert!(
                round_trip_tree.complicate(),
                "a successful simplify must be followed by a complicate transition"
            );
            let after = round_trip_tree.current();
            assert_eq!(
                after.mode, before.mode,
                "the semantic mode must survive the complicate half of the round trip"
            );
            check_case(&after).expect("the complicated case is valid");
            round_trip = true;
        }
        if observed_simplify > 0 && round_trip {
            break;
        }
    }
    assert!(
        observed_simplify > 0,
        "shrink probe: the composed strategy never simplified"
    );
    assert!(
        round_trip,
        "shrink probe: no simplify/complicate round trip was observed"
    );

    // The mode selector itself never shrinks either way.
    let mut runner = TestRunner::new(runner_config(0x5eed_c0de));
    let mut selector = proptest::sample::select(SPEED_MODES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("the mode selector produces a mode");
    let before = selector.current();
    // test-quality: determinism pin -- a persistent semantic mode must not
    // shrink (protected surface: speed-mode selection).
    assert!(!selector.simplify(), "the speed-mode selector simplified");
    assert!(
        !selector.complicate(),
        "the speed-mode selector complicated"
    );
    assert_eq!(selector.current(), before);
    println!(
        "H-OVERHANG-SPEED shrink probe: simplify={observed_simplify} \
         restores={observed_restores} round_trip={round_trip} (mode preserved)"
    );
}
