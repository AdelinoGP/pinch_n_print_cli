//! Opt-in property tests for this module's private fuzzy-skin kernel.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_12_pbt-walls`):
//!
//! * `M-FUZZY-REPEATABILITY` — deterministic seeded output for identical inputs.
//! * `M-FUZZY-SPAN-PRESERVATION` — unselected spans and endpoint metadata.
//! * `M-FUZZY-DISPLACEMENT` — independently measured offset from the source
//!   segment (Step 6).
//!
//! Step 5 implements the repeatability and ordered-span properties plus their
//! witnesses, mutations, disabled-input identity, bound matrix and shrink probe.
//! Step 6 adds the widened-f64 displacement measurement, its derived tolerance,
//! mutation control and generated property. Registry records arrive only after
//! Step 7 measures the profiles.
//!
//! Registered from `src/lib.rs` with stacked, separate `#[cfg(test)]` and
//! `#[cfg(feature = "property_testing")]` gates. The private kernel is called
//! directly; production visibility is deliberately unchanged. The `proptest_`
//! bodies require the foundation runner's `PNP_PROPERTY_CONTEXT`. No registry
//! record is added before Step 7 measures the profiles.
//!
//! # Literal review (AC-6)
//!
//! `cargo xtask check-literals` does not scan this out-of-line module source.
//! Re-derived watched types used here are `Point3WithWidth`, `WallFeatureFlags`
//! and `ExtrusionPath3D` (`crates/slicer-ir/src/slice_ir.rs`). Their literal
//! sites are respectively `point`, `flag` and `FuzzyCase::path`: points/flags use
//! FRU over `Default::default()`, and the path uses FRU over
//! `extrusion_path3d_base`. `WidthProfile` has one field and `FuzzyCase`,
//! `FuzzyOutput`, `FuzzyLimits`, `InsertedSample` and match fixtures are private
//! module-local types, not watched crate-public structs.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    units_to_mm, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, WallFeatureFlags, WidthProfile,
};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::apply_fuzzy_skin;

const REPEATABILITY_ID: &str = "M-FUZZY-REPEATABILITY";
const REPEATABILITY_TEST: &str = "property_tests::proptest_fuzzy_repeatability";
const SPAN_ID: &str = "M-FUZZY-SPAN-PRESERVATION";
const SPAN_TEST: &str = "property_tests::proptest_fuzzy_span_preservation";
const DISPLACEMENT_ID: &str = "M-FUZZY-DISPLACEMENT";
const DISPLACEMENT_TEST: &str = "property_tests::proptest_fuzzy_displacement";
const FUZZY_BOUND_KEYS: [&str; 4] = [
    "max_abs_mm_units",
    "max_thickness_units",
    "max_point_distance_units",
    "max_points",
];
const FIXED_PREFIX_LEN: usize = 3;
const MAX_GENERATED_WIDTH_UNITS: i64 = 1_000;
const SHRINK_STEPS: usize = 96;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

fn bits_eq(left: f32, right: f32) -> bool {
    left.to_bits() == right.to_bits()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApplyToAllMode {
    Disabled,
    Selected,
}

impl ApplyToAllMode {
    fn value(self) -> bool {
        matches!(self, Self::Selected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopologyMode {
    Open,
    Closed,
}

#[derive(Debug, Clone)]
struct FuzzyLimits {
    max_abs_mm_units: i64,
    max_thickness_units: i64,
    max_point_distance_units: i64,
    max_points: u32,
}

#[derive(Debug, Clone)]
struct FuzzyCase {
    points: Vec<Point3WithWidth>,
    flags: Vec<WallFeatureFlags>,
    widths: Vec<f32>,
    apply_to_all: bool,
    topology: TopologyMode,
    thickness: f32,
    point_distance: f32,
    layer_index: u32,
    wall_index: u32,
}

impl FuzzyCase {
    fn path(&self) -> ExtrusionPath3D {
        ExtrusionPath3D {
            points: self.points.clone(),
            ..extrusion_path3d_base(ExtrusionRole::OuterWall)
        }
    }

    fn width_profile(&self) -> WidthProfile {
        WidthProfile {
            widths: self.widths.clone(),
        }
    }

    fn is_closed(&self) -> bool {
        self.points.len() >= 2
            && bits_eq(self.points[0].x, self.points[self.points.len() - 1].x)
            && bits_eq(self.points[0].y, self.points[self.points.len() - 1].y)
    }
}

#[derive(Debug, Clone)]
struct FuzzyOutput {
    points: Vec<Point3WithWidth>,
    flags: Vec<WallFeatureFlags>,
    widths: Vec<f32>,
}

#[derive(Debug, Clone, Copy)]
struct InsertedSample {
    segment_index: usize,
    output_index: usize,
    point: Point3WithWidth,
}

#[derive(Debug, Clone)]
struct MatchedFuzzy {
    inserted: Vec<InsertedSample>,
    endpoints: Vec<usize>,
}

#[derive(Debug, Clone)]
struct TailVertex {
    point: Point3WithWidth,
    flag: WallFeatureFlags,
    width: f32,
    duplicate_previous: bool,
}

fn point(x_units: i64, y_units: i64, width_units: i64) -> Point3WithWidth {
    Point3WithWidth {
        x: units_to_mm(x_units),
        y: units_to_mm(y_units),
        z: 0.2,
        width: units_to_mm(width_units),
        flow_factor: 1.0,
        ..Default::default()
    }
}

fn flag(fuzzy_skin: bool, tool_index: Option<u32>, salt: bool) -> WallFeatureFlags {
    WallFeatureFlags {
        tool_index,
        fuzzy_skin,
        is_bridge: salt,
        is_thin_wall: !salt,
        skip_ironing: salt,
        ..Default::default()
    }
}

fn fixed_prefix() -> (Vec<Point3WithWidth>, Vec<WallFeatureFlags>, Vec<f32>) {
    (
        vec![
            point(0, 0, 4_000),
            point(20_000, 0, 5_000),
            point(20_000, 20_000, 4_000),
        ],
        vec![
            flag(true, None, false),
            flag(false, None, false),
            flag(false, None, false),
        ],
        vec![0.4, 0.5, 0.4],
    )
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

fn checked_limits(bounds: &BTreeMap<String, u64>) -> Result<FuzzyLimits, DomainError> {
    let max_abs_mm_units = i64::try_from(required(bounds, "max_abs_mm_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_thickness_units = i64::try_from(required(bounds, "max_thickness_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_point_distance_units = i64::try_from(required(bounds, "max_point_distance_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_points = u32::try_from(required(bounds, "max_points")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    if max_abs_mm_units < 20_000 {
        return Err(DomainError::InvalidBounds(
            "max_abs_mm_units must contain the fixed 2 mm prefix".to_owned(),
        ));
    }
    if max_thickness_units <= 0 || max_point_distance_units <= 0 {
        return Err(DomainError::InvalidBounds(
            "thickness and point_distance bounds must be positive".to_owned(),
        ));
    }
    if max_points < 3 {
        return Err(DomainError::InvalidBounds(
            "max_points must be at least 3".to_owned(),
        ));
    }
    let max_points_usize =
        usize::try_from(max_points).map_err(|_| DomainError::ArithmeticOverflow)?;
    if max_points_usize < FIXED_PREFIX_LEN {
        return Err(DomainError::InvalidBounds(
            "max_points cannot contain the fixed prefix".to_owned(),
        ));
    }
    Ok(FuzzyLimits {
        max_abs_mm_units,
        max_thickness_units,
        max_point_distance_units,
        max_points,
    })
}

fn live_limits(run: &PropertyRun) -> FuzzyLimits {
    let mut bounds = BTreeMap::new();
    for key in FUZZY_BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("fuzzy bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_limits(&bounds).expect("fuzzy bound consistency")
}

fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_mm_units".to_owned(), 20_000),
        ("max_thickness_units".to_owned(), 3_000),
        ("max_point_distance_units".to_owned(), 8_000),
        ("max_points".to_owned(), 8),
    ])
}

fn tail_vertex_strategy(limits: &FuzzyLimits) -> BoxedStrategy<TailVertex> {
    (
        -limits.max_abs_mm_units..=limits.max_abs_mm_units,
        -limits.max_abs_mm_units..=limits.max_abs_mm_units,
        1_i64..=MAX_GENERATED_WIDTH_UNITS,
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(any::<u32>()),
        1_i64..=MAX_GENERATED_WIDTH_UNITS,
    )
        .prop_map(
            |(x, y, point_width, fuzzy, salt, duplicate_previous, tool, profile_width)| {
                TailVertex {
                    point: point(x, y, point_width),
                    flag: flag(fuzzy, tool, salt),
                    width: units_to_mm(profile_width),
                    duplicate_previous,
                }
            },
        )
        .boxed()
}

fn fuzzy_strategy(
    limits: &FuzzyLimits,
    apply_modes: &[ApplyToAllMode],
    topology_modes: &[TopologyMode],
) -> BoxedStrategy<FuzzyCase> {
    assert!(!apply_modes.is_empty());
    assert!(!topology_modes.is_empty());
    let topologies: Vec<TopologyMode> = topology_modes
        .iter()
        .copied()
        .filter(|topology| *topology == TopologyMode::Open || limits.max_points >= 4)
        .collect();
    assert!(
        !topologies.is_empty(),
        "closed topology needs four points including its explicit repeat"
    );
    let selectors = (
        prop::sample::select(apply_modes.to_vec()).no_shrink(),
        prop::sample::select(topologies).no_shrink(),
        Just(fixed_prefix()),
    );
    let limits = limits.clone();
    selectors
        .prop_ind_flat_map(move |(apply, topology, prefix)| {
            let closing_points = usize::from(topology == TopologyMode::Closed);
            let max_tail = limits.max_points as usize - FIXED_PREFIX_LEN - closing_points;
            (
                proptest::collection::vec(tail_vertex_strategy(&limits), 0..=max_tail),
                1_i64..=limits.max_thickness_units,
                1_i64..=limits.max_point_distance_units,
                any::<u32>(),
                any::<u32>(),
            )
                .prop_map(move |(tail, thickness, distance, layer, wall)| {
                    let (mut points, mut flags, mut widths) = prefix.clone();
                    for vertex in tail {
                        if vertex.duplicate_previous && points.len() > FIXED_PREFIX_LEN {
                            flags[points.len() - 1].fuzzy_skin = true;
                            let previous = *points.last().expect("prefix is nonempty");
                            points.push(previous);
                        } else {
                            points.push(vertex.point);
                        }
                        flags.push(vertex.flag);
                        widths.push(vertex.width);
                    }
                    if topology == TopologyMode::Closed {
                        points.push(points[0]);
                        flags.push(flags[0].clone());
                        widths.push(widths[0]);
                    }
                    FuzzyCase {
                        points,
                        flags,
                        widths,
                        apply_to_all: apply.value(),
                        topology,
                        thickness: units_to_mm(thickness),
                        point_distance: units_to_mm(distance),
                        layer_index: layer,
                        wall_index: wall,
                    }
                })
        })
        .boxed()
}

fn run_fuzzy(case: &FuzzyCase) -> FuzzyOutput {
    let (points, flags, widths) = apply_fuzzy_skin(
        &case.path(),
        &case.flags,
        &case.width_profile(),
        case.apply_to_all,
        case.thickness,
        case.point_distance,
        case.layer_index,
        case.wall_index,
    );
    FuzzyOutput {
        points,
        flags,
        widths,
    }
}

fn point_eq(left: &Point3WithWidth, right: &Point3WithWidth) -> bool {
    bits_eq(left.x, right.x)
        && bits_eq(left.y, right.y)
        && bits_eq(left.z, right.z)
        && bits_eq(left.width, right.width)
        && bits_eq(left.flow_factor, right.flow_factor)
        && left.overhang_quartile == right.overhang_quartile
        && bits_eq(left.dist_to_top_mm, right.dist_to_top_mm)
        && left.overhang_distance_mm.map(f32::to_bits)
            == right.overhang_distance_mm.map(f32::to_bits)
}

fn output_eq(case: &FuzzyCase, output: &FuzzyOutput) -> bool {
    output.points.len() == case.points.len()
        && output.flags.len() == case.flags.len()
        && output.widths.len() == case.widths.len()
        && output
            .points
            .iter()
            .zip(&case.points)
            .all(|(left, right)| point_eq(left, right))
        && output.flags == case.flags
        && output
            .widths
            .iter()
            .zip(&case.widths)
            .all(|(left, right)| bits_eq(*left, *right))
}

fn outputs_eq(left: &FuzzyOutput, right: &FuzzyOutput) -> bool {
    left.points.len() == right.points.len()
        && left.flags.len() == right.flags.len()
        && left.widths.len() == right.widths.len()
        && left
            .points
            .iter()
            .zip(&right.points)
            .all(|(a, b)| point_eq(a, b))
        && left.flags == right.flags
        && left
            .widths
            .iter()
            .zip(&right.widths)
            .all(|(a, b)| bits_eq(*a, *b))
}

fn segment_selected(case: &FuzzyCase, segment: usize) -> bool {
    case.apply_to_all || case.flags[segment].fuzzy_skin
}

fn segment_zero(case: &FuzzyCase, segment: usize) -> bool {
    let start = case.points[segment];
    let end = case.points[segment + 1];
    bits_eq(start.x, end.x) && bits_eq(start.y, end.y)
}

fn endpoint_matches(case: &FuzzyCase, output: &FuzzyOutput, input: usize, out: usize) -> bool {
    out < output.points.len()
        && point_eq(&case.points[input], &output.points[out])
        && case.flags[input] == output.flags[out]
        && bits_eq(case.widths[input], output.widths[out])
}

#[derive(Debug, Clone)]
struct Parse {
    inserted: Vec<InsertedSample>,
    endpoints: Vec<usize>,
}

#[allow(clippy::too_many_arguments)]
fn parse_tail(
    case: &FuzzyCase,
    output: &FuzzyOutput,
    segment: usize,
    out: usize,
    memo: &mut HashMap<(usize, usize), Option<Parse>>,
) -> Option<Parse> {
    if let Some(cached) = memo.get(&(segment, out)) {
        return cached.clone();
    }
    if !endpoint_matches(case, output, segment, out) {
        memo.insert((segment, out), None);
        return None;
    }
    if segment == case.points.len() - 1 {
        let result = (out == output.points.len() - 1).then(|| Parse {
            inserted: Vec::new(),
            endpoints: vec![out],
        });
        memo.insert((segment, out), result.clone());
        return result;
    }

    let mut try_branch = |end: usize, inserted_start: usize| -> Option<Parse> {
        if !endpoint_matches(case, output, segment + 1, end) {
            return None;
        }
        for index in inserted_start..end {
            let valid_flag = output.flags[index] == case.flags[segment]
                || output.flags[index] == case.flags[segment + 1];
            let valid_width = bits_eq(output.widths[index], case.widths[segment])
                || bits_eq(output.widths[index], case.widths[segment + 1]);
            if !valid_flag || !valid_width {
                return None;
            }
        }
        let mut rest = parse_tail(case, output, segment + 1, end, memo)?;
        let mut inserted: Vec<InsertedSample> = (inserted_start..end)
            .map(|output_index| InsertedSample {
                segment_index: segment,
                output_index,
                point: output.points[output_index],
            })
            .collect();
        inserted.append(&mut rest.inserted);
        let mut endpoints = vec![out];
        endpoints.append(&mut rest.endpoints);
        Some(Parse {
            inserted,
            endpoints,
        })
    };

    let selected = segment_selected(case, segment);
    let zero = segment_zero(case, segment);
    let result = if !selected || zero {
        try_branch(out + 1, out + 1)
    } else {
        let mut found = None;
        for end in out + 2..output.points.len() {
            if let Some(parse) = try_branch(end, out + 1) {
                found = Some(parse);
                break;
            }
        }
        found
    };
    memo.insert((segment, out), result.clone());
    result
}

fn match_fuzzy_output(case: &FuzzyCase, output: &FuzzyOutput) -> Result<MatchedFuzzy, String> {
    if case.points.len() < 2 {
        return Err("span matcher needs at least two input points".to_owned());
    }
    if case.points.len() != case.flags.len() || case.points.len() != case.widths.len() {
        return Err("input arrays are not parallel".to_owned());
    }
    if output.points.len() != output.flags.len() || output.points.len() != output.widths.len() {
        return Err(format!(
            "output arrays differ: points={} flags={} widths={}",
            output.points.len(),
            output.flags.len(),
            output.widths.len()
        ));
    }
    let mut memo = HashMap::new();
    let parse = parse_tail(case, output, 0, 0, &mut memo).ok_or_else(|| {
        "no ordered, endpoint-exact parse consumes every input and output entry".to_owned()
    })?;
    Ok(MatchedFuzzy {
        inserted: parse.inserted,
        endpoints: parse.endpoints,
    })
}

fn witness_case(apply_to_all: bool) -> FuzzyCase {
    let points = vec![
        point(0, 0, 4_000),
        point(20_000, 0, 4_000),
        point(20_000, 20_000, 4_000),
        point(0, 20_000, 4_000),
        point(0, 0, 4_000),
    ];
    FuzzyCase {
        points,
        flags: [true, false, true, true, true]
            .into_iter()
            .map(|fuzzy| flag(fuzzy, None, false))
            .collect(),
        widths: vec![0.4, 0.5, 0.4, 0.45, 0.4],
        apply_to_all,
        topology: TopologyMode::Closed,
        thickness: 0.3,
        point_distance: 0.8,
        layer_index: 7,
        wall_index: 3,
    }
}

// test-quality: determinism pin — repeated identical apply_fuzzy_skin calls pin seeded stability, not geometric correctness.
#[test]
fn proptest_fuzzy_repeatability() {
    let run = PropertyRun::new(REPEATABILITY_ID, REPEATABILITY_TEST)
        .expect("M-FUZZY-REPEATABILITY registry context");
    let limits = live_limits(&run);
    let strategy = fuzzy_strategy(
        &limits,
        &[ApplyToAllMode::Disabled, ApplyToAllMode::Selected],
        &[TopologyMode::Open, TopologyMode::Closed],
    );
    let calls = std::cell::Cell::new(0_u64);
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let first = run_fuzzy(&case);
        let second = run_fuzzy(&case);
        if outputs_eq(&first, &second) {
            Ok(())
        } else {
            fail(format!(
                "identical inputs differ for layer={} wall={} apply_to_all={} topology={:?}",
                case.layer_index, case.wall_index, case.apply_to_all, case.topology
            ))
        }
    })
    .expect("M-FUZZY-REPEATABILITY deterministic output");
    println!("M-FUZZY-REPEATABILITY accepted_calls={}", calls.get());
}

#[test]
fn proptest_fuzzy_span_preservation() {
    let run = PropertyRun::new(SPAN_ID, SPAN_TEST).expect("M-FUZZY-SPAN-PRESERVATION context");
    let limits = live_limits(&run);
    let strategy = fuzzy_strategy(
        &limits,
        &[ApplyToAllMode::Disabled, ApplyToAllMode::Selected],
        &[TopologyMode::Open, TopologyMode::Closed],
    );
    let calls = std::cell::Cell::new(0_u64);
    let inserted_cases = std::cell::Cell::new(0_u64);
    let selected_zero_cases = std::cell::Cell::new(0_u64);
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        if (0..case.points.len() - 1)
            .any(|segment| segment_selected(&case, segment) && segment_zero(&case, segment))
        {
            selected_zero_cases.set(selected_zero_cases.get() + 1);
        }
        let output = run_fuzzy(&case);
        match match_fuzzy_output(&case, &output) {
            Ok(matched) => {
                for sample in &matched.inserted {
                    if sample.output_index >= output.points.len()
                        || !point_eq(&output.points[sample.output_index], &sample.point)
                    {
                        return fail(format!(
                            "matcher sample {}/{} does not identify its real output point",
                            sample.segment_index, sample.output_index
                        ));
                    }
                }
                if matched
                    .inserted
                    .iter()
                    .any(|sample| sample.segment_index == 0)
                {
                    inserted_cases.set(inserted_cases.get() + 1);
                    Ok(())
                } else {
                    fail("the fixed selected leading segment produced no inserted sample")
                }
            }
            Err(message) => fail(format!(
                "span contract failed (apply_to_all={} topology={:?}): {message}",
                case.apply_to_all, case.topology
            )),
        }
    })
    .expect("M-FUZZY-SPAN-PRESERVATION ordered spans");
    assert!(inserted_cases.get() > 0);
    assert!(
        selected_zero_cases.get() > 0,
        "the generated population never exercised a selected zero-length segment"
    );
    println!(
        "M-FUZZY-SPAN-PRESERVATION accepted_calls={} leading_inserted_cases={} \
         selected_zero_length_cases={}",
        calls.get(),
        inserted_cases.get(),
        selected_zero_cases.get()
    );
}

#[test]
fn fuzzy_required_witnesses_are_nonempty() {
    let mixed = witness_case(false);
    let mixed_output = run_fuzzy(&mixed);
    let matched = match_fuzzy_output(&mixed, &mixed_output).expect("mixed witness parse");
    for segment in [0, 2, 3] {
        assert!(matched
            .inserted
            .iter()
            .any(|sample| sample.segment_index == segment));
    }
    assert!(!matched
        .inserted
        .iter()
        .any(|sample| sample.segment_index == 1));
    let endpoint = matched.endpoints[2];
    assert!(point_eq(&mixed.points[2], &mixed_output.points[endpoint]));
    assert_eq!(mixed.flags[2], mixed_output.flags[endpoint]);
    assert!(bits_eq(mixed.widths[2], mixed_output.widths[endpoint]));

    // A selected zero-length segment must insert no sample while still copying
    // its endpoint exactly. The duplicate follows the selected p2 vertex, so
    // segment 2 has a fuzzy-skin start flag and zero length by construction.
    let mut zero_length = mixed.clone();
    zero_length.points.insert(3, zero_length.points[2]);
    zero_length.flags.insert(3, flag(true, None, false));
    zero_length.widths.insert(3, zero_length.widths[2]);
    let zero_output = run_fuzzy(&zero_length);
    let zero_matched =
        match_fuzzy_output(&zero_length, &zero_output).expect("selected zero-length witness parse");
    assert!(
        !zero_matched
            .inserted
            .iter()
            .any(|sample| sample.segment_index == 2),
        "selected zero-length segment inserted a sample"
    );
    let duplicate_endpoint = zero_matched.endpoints[3];
    assert!(point_eq(
        &zero_length.points[3],
        &zero_output.points[duplicate_endpoint]
    ));
    assert!(point_eq(
        &zero_output.points[zero_matched.endpoints[2]],
        &zero_output.points[duplicate_endpoint]
    ));

    let all = witness_case(true);
    let all_matched = match_fuzzy_output(&all, &run_fuzzy(&all)).expect("all-selected parse");
    for segment in 0..4 {
        assert!(all_matched
            .inserted
            .iter()
            .any(|sample| sample.segment_index == segment));
    }

    let mut disabled = witness_case(false);
    for flag in &mut disabled.flags {
        flag.fuzzy_skin = false;
    }
    let copy = run_fuzzy(&disabled);
    assert!(output_eq(&disabled, &copy));
}

// test-quality: determinism pin — this control validates the bitwise determinism comparator and seed sensitivity, not geometric correctness.
#[test]
fn fuzzy_repeatability_control() {
    let case = witness_case(false);
    let first = run_fuzzy(&case);
    let second = run_fuzzy(&case);
    assert!(outputs_eq(&first, &second));
    assert!(!first.points.is_empty());
    let mut mutant = first.clone();
    mutant.points[1].x = f32::from_bits(mutant.points[1].x.to_bits() ^ 1);
    assert!(!outputs_eq(&first, &mutant));

    let mut changed = case.clone();
    changed.layer_index = changed.layer_index.wrapping_add(1);
    let changed_first = run_fuzzy(&changed);
    let changed_second = run_fuzzy(&changed);
    assert!(outputs_eq(&changed_first, &changed_second));
    assert!(!outputs_eq(&first, &changed_first));
}

#[test]
fn fuzzy_span_oracle_rejects_mutations() {
    let case = witness_case(false);
    let baseline = run_fuzzy(&case);
    let matched = match_fuzzy_output(&case, &baseline).expect("real baseline parse");
    let expect_rejected = |label: &str, mutant: FuzzyOutput| {
        assert!(!outputs_eq(&baseline, &mutant), "{label} made no change");
        assert!(
            match_fuzzy_output(&case, &mutant).is_err(),
            "{label} was accepted"
        );
    };

    let mut moved = baseline.clone();
    let unselected_endpoint = matched.endpoints[2];
    moved.points[unselected_endpoint].x += 0.125;
    expect_rejected("moved unselected endpoint", moved);

    // An extra point exactly at a selected/unselected boundary is inherently
    // ambiguous: a parser can attribute it to the adjacent selected segment.
    // Use an all-unselected real output, where no selected span exists and the
    // extra point therefore has no legal owner.
    let mut all_unselected = witness_case(false);
    for flag in &mut all_unselected.flags {
        flag.fuzzy_skin = false;
    }
    let unselected_baseline = run_fuzzy(&all_unselected);
    assert!(output_eq(&all_unselected, &unselected_baseline));
    let mut extra = unselected_baseline.clone();
    extra.points.insert(1, all_unselected.points[0]);
    extra.flags.insert(1, all_unselected.flags[0].clone());
    extra.widths.insert(1, all_unselected.widths[0]);
    assert!(!outputs_eq(&unselected_baseline, &extra));
    assert!(
        match_fuzzy_output(&all_unselected, &extra).is_err(),
        "inserted point in an unambiguous unselected-only output was accepted"
    );

    let mut removed = baseline.clone();
    let start = matched.endpoints[0] + 1;
    let end = matched.endpoints[1];
    assert!(start < end, "selected witness needs real inserted samples");
    removed.points.drain(start..end);
    removed.flags.drain(start..end);
    removed.widths.drain(start..end);
    expect_rejected("selected span with samples removed", removed);
}

#[test]
fn fuzzy_disabled_parameters_are_identity() {
    let case = witness_case(false);
    for invalid in [0.0, -0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut changed = case.clone();
        changed.thickness = invalid;
        assert!(output_eq(&changed, &run_fuzzy(&changed)));
        let mut changed = case.clone();
        changed.point_distance = invalid;
        assert!(output_eq(&changed, &run_fuzzy(&changed)));
    }
    for len in [0, 1] {
        let mut short = case.clone();
        short.points.truncate(len);
        short.flags.truncate(len);
        short.widths.truncate(len);
        assert!(output_eq(&short, &run_fuzzy(&short)));
    }
}

#[test]
fn fuzzy_property_domains_reject_unsafe_bounds() {
    fn invalid(label: &str, bounds: BTreeMap<String, u64>) {
        match checked_limits(&bounds) {
            Err(DomainError::InvalidBounds(message)) => assert!(!message.is_empty(), "{label}"),
            other => panic!("{label}: expected InvalidBounds, got {other:?}"),
        }
    }
    let valid = probe_bound_map();
    checked_limits(&valid).expect("valid fuzzy bounds");
    for key in FUZZY_BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        invalid("missing key", bounds);
    }
    for (key, value) in [
        ("max_abs_mm_units", 0),
        ("max_thickness_units", 0),
        ("max_point_distance_units", 0),
        ("max_points", 2),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        invalid("invalid minimum", bounds);
    }
    for key in [
        "max_abs_mm_units",
        "max_thickness_units",
        "max_point_distance_units",
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), i64::MAX as u64 + 1);
        assert!(matches!(
            checked_limits(&bounds),
            Err(DomainError::ArithmeticOverflow)
        ));
    }
    let mut oversized_count = valid;
    oversized_count.insert("max_points".to_owned(), u32::MAX as u64 + 1);
    assert!(matches!(
        checked_limits(&oversized_count),
        Err(DomainError::ArithmeticOverflow)
    ));
}

fn ordinary_bits(case: &FuzzyCase) -> Vec<u64> {
    let mut bits = vec![
        u64::from(case.thickness.to_bits()),
        u64::from(case.point_distance.to_bits()),
        u64::from(case.layer_index),
        u64::from(case.wall_index),
    ];
    for index in FIXED_PREFIX_LEN..case.points.len() {
        bits.extend([
            u64::from(case.points[index].x.to_bits()),
            u64::from(case.points[index].y.to_bits()),
            u64::from(case.points[index].width.to_bits()),
            u64::from(case.widths[index].to_bits()),
            u64::from(case.flags[index].fuzzy_skin),
        ]);
    }
    bits
}

fn assert_no_shrink<S: Strategy>(runner: &mut TestRunner, strategy: &S, label: &str)
where
    S::Value: std::fmt::Debug,
{
    let mut tree = strategy.new_tree(runner).expect("no-shrink tree");
    let before = format!("{:?}", tree.current());
    assert!(!tree.simplify(), "{label} simplified");
    assert!(!tree.complicate(), "{label} complicated");
    assert_eq!(format!("{:?}", tree.current()), before);
}

fn check_shrink_case(case: &FuzzyCase, apply: ApplyToAllMode, topology: TopologyMode) {
    assert_eq!(case.apply_to_all, apply.value());
    assert_eq!(case.topology, topology);
    assert_eq!(case.points.len(), case.flags.len());
    assert_eq!(case.points.len(), case.widths.len());
    let (points, flags, widths) = fixed_prefix();
    for index in 0..FIXED_PREFIX_LEN {
        assert!(point_eq(&case.points[index], &points[index]));
        assert_eq!(case.flags[index], flags[index]);
        assert!(bits_eq(case.widths[index], widths[index]));
    }
    assert_eq!(case.is_closed(), topology == TopologyMode::Closed);
    let output = run_fuzzy(case);
    match_fuzzy_output(case, &output).expect("shrunk fuzzy output parse");
    let output_closed = output.points.len() >= 2
        && bits_eq(output.points[0].x, output.points[output.points.len() - 1].x)
        && bits_eq(output.points[0].y, output.points[output.points.len() - 1].y);
    assert_eq!(output_closed, topology == TopologyMode::Closed);
}

#[test]
fn fuzzy_shrink_preserves_modes_and_fixed_prefix() {
    let limits = checked_limits(&probe_bound_map()).expect("shrink bounds");
    let mut runner = TestRunner::deterministic();
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(vec![ApplyToAllMode::Disabled, ApplyToAllMode::Selected]).no_shrink(),
        "apply_to_all",
    );
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(vec![TopologyMode::Open, TopologyMode::Closed]).no_shrink(),
        "topology",
    );
    for apply in [ApplyToAllMode::Disabled, ApplyToAllMode::Selected] {
        for topology in [TopologyMode::Open, TopologyMode::Closed] {
            let strategy = fuzzy_strategy(&limits, &[apply], &[topology]);
            let mut tree = strategy.new_tree(&mut runner).expect("fuzzy shrink tree");
            let initial = tree.current();
            check_shrink_case(&initial, apply, topology);
            let initial_bits = ordinary_bits(&initial);
            let mut simplified = 0_usize;
            let mut complicated = 0_usize;
            let mut changed = false;
            for _ in 0..SHRINK_STEPS {
                if !tree.simplify() {
                    break;
                }
                simplified += 1;
                let current = tree.current();
                check_shrink_case(&current, apply, topology);
                changed |= ordinary_bits(&current) != initial_bits;
                assert!(
                    tree.complicate(),
                    "complicate must restore a simplification"
                );
                complicated += 1;
                let restored = tree.current();
                check_shrink_case(&restored, apply, topology);
                changed |= ordinary_bits(&restored) != initial_bits;
            }
            assert!(simplified > 0 && complicated > 0, "false-only shrink probe");
            assert!(changed, "no ordinary fuzzy tail/parameter changed");
            println!(
                "apply={apply:?} topology={topology:?} simplify={simplified} complicate={complicated}"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M-FUZZY-DISPLACEMENT — independent measurement in widened f64.
// ─────────────────────────────────────────────────────────────────────────────

/// Derived coefficient for the f32 SUT operation sequence (`dx`/`dy`,
/// `seg_len`, normalized perpendicular, `base`, displacement multiply and final
/// add) plus the oracle's f64 cross/dot measurement. See the evidence file for
/// the term-by-term derivation; this is not an observed-output epsilon.
const DISPLACEMENT_K: f64 = 32.0;

fn fuzzy_displacement_tolerance(max_abs_mm: f64, thickness_mm: f64) -> f64 {
    DISPLACEMENT_K * f64::from(f32::EPSILON) * (max_abs_mm + thickness_mm + 1.0)
}

#[derive(Debug, Clone, Copy)]
struct DisplacementMeasurement {
    distance: f64,
    projection: f64,
    segment_length: f64,
}

fn fuzzy_measure_displacement(
    point: &Point3WithWidth,
    start: &Point3WithWidth,
    end: &Point3WithWidth,
) -> Result<DisplacementMeasurement, String> {
    let px = f64::from(point.x);
    let py = f64::from(point.y);
    let x0 = f64::from(start.x);
    let y0 = f64::from(start.y);
    let x1 = f64::from(end.x);
    let y1 = f64::from(end.y);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let segment_length = (dx * dx + dy * dy).sqrt();
    if !segment_length.is_finite() || segment_length < 1e-8 {
        return Err(format!("degenerate source segment length {segment_length}"));
    }
    let wx = px - x0;
    let wy = py - y0;
    let distance = ((wx * dy - wy * dx) / segment_length).abs();
    let projection = (wx * dx + wy * dy) / (segment_length * segment_length);
    if !distance.is_finite() || !projection.is_finite() {
        return Err("nonfinite displacement measurement".to_owned());
    }
    Ok(DisplacementMeasurement {
        distance,
        projection,
        segment_length,
    })
}

fn check_inserted_sample(
    case: &FuzzyCase,
    sample: &InsertedSample,
    max_abs_mm: f64,
) -> Result<(), String> {
    if sample.segment_index + 1 >= case.points.len() {
        return Err(format!(
            "sample segment {} is outside the path",
            sample.segment_index
        ));
    }
    let measurement = fuzzy_measure_displacement(
        &sample.point,
        &case.points[sample.segment_index],
        &case.points[sample.segment_index + 1],
    )?;
    let tolerance = fuzzy_displacement_tolerance(max_abs_mm, f64::from(case.thickness));
    let allowed = f64::from(case.thickness) + tolerance;
    let t_tolerance = tolerance / measurement.segment_length + 8.0 * f64::EPSILON;
    if measurement.distance > allowed {
        return Err(format!(
            "segment {} sample {} distance {:?} exceeds thickness+tolerance {allowed:?}",
            sample.segment_index, sample.output_index, measurement.distance
        ));
    }
    if measurement.projection < -t_tolerance || measurement.projection > 1.0 + t_tolerance {
        return Err(format!(
            "segment {} sample {} projection {:?} is outside [-{t_tolerance:?}, {}]",
            sample.segment_index,
            sample.output_index,
            measurement.projection,
            1.0 + t_tolerance
        ));
    }
    Ok(())
}

fn check_inserted_displacement(
    case: &FuzzyCase,
    matched: &MatchedFuzzy,
    max_abs_mm: f64,
) -> Result<usize, String> {
    for sample in &matched.inserted {
        check_inserted_sample(case, sample, max_abs_mm)?;
    }
    Ok(matched.inserted.len())
}

#[test]
fn proptest_fuzzy_displacement() {
    let run = PropertyRun::new(DISPLACEMENT_ID, DISPLACEMENT_TEST)
        .expect("M-FUZZY-DISPLACEMENT registry context");
    let limits = live_limits(&run);
    let strategy = fuzzy_strategy(
        &limits,
        &[ApplyToAllMode::Disabled, ApplyToAllMode::Selected],
        &[TopologyMode::Open, TopologyMode::Closed],
    );
    let max_abs_mm = f64::from(units_to_mm(limits.max_abs_mm_units));
    let calls = std::cell::Cell::new(0_u64);
    let samples = std::cell::Cell::new(0_u64);
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let output = run_fuzzy(&case);
        match match_fuzzy_output(&case, &output) {
            Ok(matched) => match check_inserted_displacement(&case, &matched, max_abs_mm) {
                Ok(count) => {
                    samples.set(samples.get() + count as u64);
                    if count > 0 {
                        Ok(())
                    } else {
                        fail("a valid case produced no inserted sample to measure")
                    }
                }
                Err(message) => fail(message),
            },
            Err(message) => fail(message),
        }
    })
    .expect("M-FUZZY-DISPLACEMENT offset measurement");
    assert!(samples.get() > 0);
    println!(
        "M-FUZZY-DISPLACEMENT accepted_calls={} inserted_samples={}",
        calls.get(),
        samples.get()
    );
}

#[test]
fn fuzzy_displacement_oracle_rejects_mutations() {
    let case = witness_case(false);
    let output = run_fuzzy(&case);
    let matched = match_fuzzy_output(&case, &output).expect("real displacement baseline");
    check_inserted_displacement(&case, &matched, 2.0).expect("real baseline within envelope");
    let closing = *matched
        .inserted
        .iter()
        .find(|sample| sample.segment_index == 3)
        .expect("the closing edge must have samples");
    let start = case.points[3];
    let end = case.points[4];
    let measurement =
        fuzzy_measure_displacement(&closing.point, &start, &end).expect("measurement");
    assert!(
        measurement.distance > 0.0,
        "the deterministic closing-edge witness needs positive displacement"
    );

    let dx = f64::from(end.x) - f64::from(start.x);
    let dy = f64::from(end.y) - f64::from(start.y);
    let length = (dx * dx + dy * dy).sqrt();
    let tolerance = fuzzy_displacement_tolerance(2.0, f64::from(case.thickness));

    let mut too_far = closing;
    let signed_before = ((f64::from(closing.point.x) - f64::from(start.x)) * dy
        - (f64::from(closing.point.y) - f64::from(start.y)) * dx)
        / length;
    let sign = if signed_before < 0.0 { -1.0 } else { 1.0 };
    let outward = f64::from(case.thickness) + tolerance + 0.1;
    too_far.point.x = (f64::from(too_far.point.x) - sign * dy / length * outward) as f32;
    too_far.point.y = (f64::from(too_far.point.y) + sign * dx / length * outward) as f32;
    let moved = fuzzy_measure_displacement(&too_far.point, &start, &end).expect("measurement");
    assert!(
        moved.distance > f64::from(case.thickness) + tolerance,
        "mutation did not move the sample beyond the envelope: {moved:?}"
    );
    let rejected = check_inserted_sample(&case, &too_far, 2.0);
    assert!(
        rejected.is_err(),
        "over-thickness displacement was accepted"
    );

    let mut too_long = closing;
    too_long.point.x = (f64::from(start.x) - dx / length * 0.5) as f32;
    too_long.point.y = (f64::from(start.y) - dy / length * 0.5) as f32;
    let rejected = check_inserted_sample(&case, &too_long, 2.0);
    assert!(rejected.is_err(), "beyond-span projection was accepted");

    let wrong = matched
        .inserted
        .iter()
        .filter(|sample| sample.segment_index == 3)
        .find(|sample| {
            let wrong = InsertedSample {
                segment_index: 0,
                output_index: sample.output_index,
                point: sample.point,
            };
            check_inserted_sample(&case, &wrong, 2.0).is_err()
        })
        .expect("a closing-edge sample must not satisfy the orthogonal first segment");
    assert!(
        check_inserted_sample(
            &case,
            &InsertedSample {
                segment_index: 0,
                output_index: wrong.output_index,
                point: wrong.point,
            },
            2.0
        )
        .is_err(),
        "wrong-segment attribution was accepted"
    );
}
