//! Deterministic controls and concrete regressions for `H-ARACHNE-BEADING`.
//!
//! Nothing here reads `PNP_PROPERTY_CONTEXT` or uses any RNG. Every control
//! writes its input and expected value here, calls the real SUT on the named
//! fixture (which must pass the comparator), and then feeds a deliberately
//! broken result or an independently invalid assertion through **the same
//! comparator the registry property uses** (which must fail).

use slicer_core::beading::distributed::DistributedBeadingStrategy;
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::beading::limited::LimitedBeadingStrategy;
use slicer_core::beading::widening::WideningBeadingStrategy;
use slicer_core::beading::BeadingStrategy;
use slicer_core::skeletal_trapezoidation::{
    populate_beading_propagation, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::Vertex;
use slicer_ir::UNITS_PER_MM;

use super::beading::{
    check_case, check_conservation, check_parallel, compare_limited, strategy, to_f64_exact,
    to_usize, validate_limits, witness_cap, BeadingCase, BeadingLimits, BoundsError, Envelope,
    Envelopes, LimitedExpectation, OracleError, RawBeadingBounds, MIDDLE_THRESHOLD,
    WALL_TRANSITION_ANGLE,
};
use super::graph::{validate_graph_bounds, RawGraphBounds};
use super::pipeline::{validate_pipeline_bounds, RawPipelineBounds};

/// Distributed parent with `distribution_count = 2` (the `radius >= 2` weight
/// branch) and the charter's fixed middle threshold.
fn distributed(width: f64) -> DistributedBeadingStrategy {
    DistributedBeadingStrategy::new(
        width,
        0.0,
        0.0,
        2,
        WALL_TRANSITION_ANGLE,
        MIDDLE_THRESHOLD,
        MIDDLE_THRESHOLD,
    )
}

/// A valid reference profile, built directly (never from the environment). It
/// is the same validator the property calls, with authoring-tier values.
fn reference_raw_bounds() -> RawBeadingBounds {
    RawBeadingBounds {
        max_beads: 8,
        max_width_units: 20_000,
        max_excess_units: 8_000,
    }
}

fn reference_limits() -> BeadingLimits {
    validate_limits(reference_raw_bounds()).expect("reference profile")
}

fn reference_envelopes() -> Envelopes {
    Envelopes::derive(&reference_limits()).expect("reference envelopes")
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-1 / AC-N2 controls: the comparator rejects what the charter must catch.
// ─────────────────────────────────────────────────────────────────────────────

/// Prove the independent limited-beating comparator accepts the real SUT and
/// rejects a lost sentinel and a non-parallel array pair, and prove the
/// conservation identity is genuinely invalid for widening output.
pub fn oracle_controls() {
    let envelopes = reference_envelopes();
    let width = 4000.0_f64;
    let cap = 4_usize;
    let half = cap / 2;
    let excess = 2000.0_f64;
    let at_cap_thickness = cap as f64 * width;
    let over_cap_thickness = at_cap_thickness + excess;
    let limited = LimitedBeadingStrategy::new(Box::new(distributed(width)), cap);

    // ── The real SUT passes the comparator at the cap and one over it ─────
    let at_cap = limited.compute(at_cap_thickness, cap);
    let at_cap_expectation = LimitedExpectation {
        cap,
        half,
        bead_width: width,
        thickness: at_cap_thickness,
        sentinels: 1,
        left_over: 0.0,
    };
    compare_limited(
        &at_cap,
        &at_cap_expectation,
        &envelopes.width,
        &envelopes.location,
    )
    .expect("the real limited output at an even cap satisfies the derived model");

    let over_cap = limited.compute(over_cap_thickness, cap + 1);
    let over_cap_expectation = LimitedExpectation {
        cap,
        half,
        bead_width: width,
        thickness: over_cap_thickness,
        sentinels: 2,
        left_over: excess,
    };
    compare_limited(
        &over_cap,
        &over_cap_expectation,
        &envelopes.width,
        &envelopes.location,
    )
    .expect("the real limited output one over an even cap satisfies the derived model");

    // The named population must be the exact shape the charter claims, so the
    // "passes" above cannot be a degenerate agreement between two empty lists.
    assert_eq!(
        over_cap.bead_widths,
        vec![4000.0, 4000.0, 0.0, 0.0, 4000.0, 4000.0],
        "over-cap limited beading must keep 2k real beads and 2 markers"
    );
    assert_eq!(
        over_cap.toolpath_locations,
        vec![2000.0, 6000.0, 8000.0, 10000.0, 12000.0, 16000.0],
        "over-cap limited centres: equal spacing left, mirrored right, markers at k*w and t-k*w"
    );
    assert_eq!(
        over_cap.left_over, excess,
        "over-cap left_over is the surplus"
    );

    // ── A lost sentinel pair is rejected ───────────────────────────────────
    // Removing **both** the width and its location keeps the arrays parallel,
    // so this exercises the marker contract rather than the length contract.
    let mut lost = over_cap.clone();
    lost.bead_widths.remove(half);
    lost.toolpath_locations.remove(half);
    lost.bead_widths.remove(half);
    lost.toolpath_locations.remove(half);
    let error = compare_limited(
        &lost,
        &over_cap_expectation,
        &envelopes.width,
        &envelopes.location,
    )
    .expect_err("a limited beading that lost both markers must be rejected");
    assert_eq!(
        error,
        OracleError::SentinelCount {
            expected: 2,
            observed: 0,
        },
        "the rejection must name the marker contract"
    );

    // ── A non-parallel width/location pair is rejected ─────────────────────
    let mut skewed = over_cap.clone();
    skewed.toolpath_locations.pop();
    let error = compare_limited(
        &skewed,
        &over_cap_expectation,
        &envelopes.width,
        &envelopes.location,
    )
    .expect_err("a beading with mismatched array lengths must be rejected");
    assert!(
        matches!(
            error,
            OracleError::LengthMismatch {
                widths: 6,
                locations: 5
            }
        ),
        "the rejection must name the parallel-array contract, got {error}"
    );

    // ── Conservation is valid for distributed/limited, invalid for widening ─
    check_conservation(&over_cap, &envelopes.width)
        .expect("the real over-cap limited output does conserve thickness");
    check_conservation(&distributed(width).compute(12_000.0, 3), &envelopes.width)
        .expect("the real distributed three-bead output does conserve thickness");

    let widening =
        WideningBeadingStrategy::new(Box::new(distributed(width)), width, 1000.0, 3400.0);
    let widened = widening.compute(3000.0, 1);
    check_parallel(&widened).expect("the real widening output is still structurally parallel");
    assert_eq!(
        widened.bead_widths,
        vec![3400.0],
        "the single-bead widening branch clamps 3000 up to min_output_width"
    );
    let over_emit = widened.bead_widths.iter().sum::<f64>() - widened.total_thickness;
    assert!(
        over_emit > 0.0,
        "the widening fixture must genuinely emit more width than the region thickness"
    );
    let error = check_conservation(&widened, &envelopes.width)
        .expect_err("conservation must not hold for widening output");
    assert!(
        matches!(error, OracleError::Conservation { .. }),
        "the rejection must name the conservation identity, got {error}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-N1: unusable profiles fail before any strategy is sampled.
// ─────────────────────────────────────────────────────────────────────────────

/// Prove zero, non-representable, out-of-range and impossible-witness bounds
/// are rejected by the exact validators the property calls, before any value
/// tree exists, and that valid bounds convert losslessly.
pub fn domain_rejections() {
    let good = reference_raw_bounds();
    let limits = reference_limits();
    assert_eq!(limits.max_beads as u64, good.max_beads);
    assert_eq!(limits.max_width_units, good.max_width_units);
    assert_eq!(limits.max_excess_units, good.max_excess_units);
    assert_eq!(limits.max_width, good.max_width_units as f64);
    assert_eq!(limits.max_excess, good.max_excess_units as f64);
    assert!(
        strategy(&limits).is_ok(),
        "the reference profile must produce a value tree"
    );

    let reject = |label: &str, raw: RawBeadingBounds| -> BoundsError {
        match validate_limits(raw) {
            Ok(limits) => panic!("{label}: invalid profile was accepted as {limits:?}"),
            Err(error) => {
                assert!(!error.to_string().is_empty(), "{label}: empty diagnostic");
                error
            }
        }
    };

    // Zero, non-representable and out-of-range bounds.
    assert_eq!(
        reject(
            "zero max_beads",
            RawBeadingBounds {
                max_beads: 0,
                ..good
            }
        ),
        BoundsError::Zero {
            name: "max_beads".to_string()
        }
    );
    assert!(
        matches!(
            reject(
                "max_beads above the ceiling",
                RawBeadingBounds {
                    max_beads: u64::MAX,
                    ..good
                }
            ),
            BoundsError::OutOfRange { .. }
        ),
        "an unchecked count must be rejected by the ceiling, never narrowed"
    );
    assert_eq!(
        reject(
            "zero max_width_units",
            RawBeadingBounds {
                max_width_units: 0,
                ..good
            }
        ),
        BoundsError::Zero {
            name: "max_width_units".to_string()
        }
    );
    assert!(
        matches!(
            reject(
                "non-representable max_width_units",
                RawBeadingBounds {
                    max_width_units: u64::MAX,
                    ..good
                }
            ),
            BoundsError::Inexact { target: "f64", .. }
        ),
        "a non-representable length must be rejected, never rounded"
    );
    assert_eq!(
        reject(
            "zero max_excess_units",
            RawBeadingBounds {
                max_excess_units: 0,
                ..good
            }
        ),
        BoundsError::Zero {
            name: "max_excess_units".to_string()
        }
    );
    assert!(
        matches!(
            reject(
                "derived product above 2^53",
                RawBeadingBounds {
                    max_beads: 64,
                    max_width_units: 1 << 50,
                    ..good
                }
            ),
            BoundsError::Arithmetic { .. }
        ),
        "a profile whose derived products leave the exact range must be rejected"
    );

    // Checked conversions are exact or errors; no saturating or narrowing cast.
    assert_eq!(
        to_f64_exact("max_width_units", 1 << 53),
        Ok(9_007_199_254_740_992.0)
    );
    assert!(matches!(
        to_f64_exact("max_width_units", (1 << 53) + 1),
        Err(BoundsError::Inexact { target: "f64", .. })
    ));
    assert!(matches!(
        to_usize("max_beads", 0),
        Err(BoundsError::Zero { .. })
    ));

    // Impossible witness bounds: a `max_beads` too small for the mandatory
    // three-bead distributed case, and a cap with no room for `cap + 1`.
    assert!(
        matches!(
            reject(
                "two max_beads",
                RawBeadingBounds {
                    max_beads: 2,
                    ..good
                }
            ),
            BoundsError::BelowWitness { minimum: 3, .. }
        ),
        "max_beads = 2 cannot carry the distributed three-bead witness"
    );
    assert!(
        matches!(
            witness_cap(2),
            Err(BoundsError::BelowWitness { minimum: 3, .. })
        ),
        "an even cap of 2 cannot also carry the requested cap + 1 case"
    );
    assert_eq!(
        witness_cap(4),
        Ok((2, 3)),
        "max_beads = 4 can only carry cap 2 plus the requested 3, not cap 4 plus 5"
    );
    assert_eq!(witness_cap(8), Ok((6, 7)));
    assert_eq!(witness_cap(3), Ok((2, 3)));

    // ── H-ARACHNE-GRAPH / H-ARACHNE-PIPELINE profile minimums ──────────────
    // The same rule applies to the Step 4 charters: prove every mandatory
    // witness minimum before any strategy exists, and reject impossible or
    // overflowing profiles instead of narrowing them.
    // exhaustive: a complete authoring profile, never a helper default
    let good_graph = RawGraphBounds {
        max_abs_units: 400_000,
        max_vertices: 24,
        max_holes: 2,
        max_components: 3,
        max_shrink_candidates: 64,
    };
    let graph_limits = validate_graph_bounds(good_graph).expect("reference graph profile");
    assert!(
        graph_limits.max_abs_units >= 100_000,
        "10 mm rectangle witness"
    );
    assert!(graph_limits.max_vertices >= 8, "Holed/Thin vertex minimum");
    assert!(graph_limits.max_holes >= 1, "Holed witness minimum");
    assert!(
        graph_limits.max_components >= 2,
        "Disconnected witness minimum"
    );
    assert!(graph_limits.max_shrink_candidates >= 1);
    assert_eq!(
        graph_limits.catalog_abs_units, 160,
        "400_000 / SCALE(2_500) leaves the Thin template's requested half-span"
    );

    let reject_graph = |label: &str, raw: RawGraphBounds| -> String {
        match validate_graph_bounds(raw) {
            Ok(limits) => panic!("{label}: invalid graph profile accepted as {limits:?}"),
            Err(error) => {
                assert!(!error.to_string().is_empty(), "{label}: empty diagnostic");
                error.to_string()
            }
        }
    };
    assert!(
        reject_graph(
            "max_abs_units below the 10 mm witness",
            RawGraphBounds {
                max_abs_units: 99_999,
                ..good_graph
            }
        )
        .contains("max_abs_units"),
        "the 10 mm square witness must be admitted by the bound"
    );
    assert!(reject_graph(
        "max_vertices below the Holed template",
        RawGraphBounds {
            max_vertices: 7,
            ..good_graph
        }
    )
    .contains("max_vertices"));
    assert!(reject_graph(
        "zero max_vertices",
        RawGraphBounds {
            max_vertices: 0,
            ..good_graph
        }
    )
    .contains("max_vertices"));
    assert!(reject_graph(
        "zero max_holes",
        RawGraphBounds {
            max_holes: 0,
            ..good_graph
        }
    )
    .contains("max_holes"));
    assert!(reject_graph(
        "single max_components",
        RawGraphBounds {
            max_components: 1,
            ..good_graph
        }
    )
    .contains("max_components"));
    assert!(reject_graph(
        "zero max_shrink_candidates",
        RawGraphBounds {
            max_shrink_candidates: 0,
            ..good_graph
        }
    )
    .contains("max_shrink_candidates"));
    assert!(
        reject_graph(
            "overflowing max_abs_units",
            RawGraphBounds {
                max_abs_units: u64::MAX,
                ..good_graph
            }
        )
        .contains("max_abs_units"),
        "an unchecked bound must be rejected, never narrowed"
    );

    // exhaustive: a complete authoring pipeline profile
    let good_pipeline = RawPipelineBounds {
        geometry: good_graph,
        max_beads: 10,
    };
    let pipeline_limits =
        validate_pipeline_bounds(good_pipeline).expect("reference pipeline profile");
    assert_eq!(
        pipeline_limits.max_bead_cap, 10,
        "the ArachneParams::default cap the square witness runs with"
    );
    for bad in [0_u64, 9, 65, u64::MAX] {
        assert!(
            validate_pipeline_bounds(RawPipelineBounds {
                max_beads: bad,
                ..good_pipeline
            })
            .is_err(),
            "max_beads = {bad} must be rejected before sampling"
        );
    }
    assert_eq!(
        validate_pipeline_bounds(RawPipelineBounds {
            max_beads: 11,
            ..good_pipeline
        })
        .expect("odd max_beads is narrowed to the nearest even cap")
        .max_bead_cap,
        10,
        "canonical caps are even; 11 becomes 10, never 12"
    );
    assert!(
        validate_pipeline_bounds(RawPipelineBounds {
            geometry: RawGraphBounds {
                max_vertices: 7,
                ..good_graph
            },
            ..good_pipeline
        })
        .is_err(),
        "the pipeline inherits the graph witness minimums"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic smoke of the property body (not a property execution).
// ─────────────────────────────────────────────────────────────────────────────

/// Run the registry property's own per-case checker on three hand-built cases.
///
/// This is **not** a registry execution and is never credited as one: the
/// registry record does not exist yet, and `proptest_arachne_beading_relations`
/// correctly fails with `missing_property_context` under a plain `cargo test`.
/// Its purpose is to prove the body the registry will run is reachable,
/// non-vacuous and green on deterministic inputs covering the branch extremes:
/// an integer width at the reference scale, a shifted dyadic width with the
/// largest even cap and the largest `n`, and a sub-unit width with the
/// smallest cap.
pub fn beading_body_smoke() {
    let limits = reference_limits();
    let envelopes = reference_envelopes();
    let cases = [
        // w = 4000, n = 3, cap = 4, surplus 2000.
        BeadingCase {
            width_units: 4000,
            shift: 0,
            distributed_extra: 0,
            thickness_units: 5000,
            distribution_count: 2,
            cap_half: 2,
            excess_units: 2000,
            widening_step: 1,
            widening_scale: 4,
        },
        // w = 3001 / 8 = 375.125, n = 8, cap = 6, surplus 8000.
        BeadingCase {
            width_units: 3001,
            shift: 3,
            distributed_extra: 5,
            thickness_units: 1234,
            distribution_count: 1,
            cap_half: 3,
            excess_units: 8000,
            widening_step: 7,
            widening_scale: 15,
        },
        // w = 1 / 8 = 0.125, n = 3, cap = 2, surplus 1 / 8.
        BeadingCase {
            width_units: 1,
            shift: 3,
            distributed_extra: 0,
            thickness_units: 1,
            distribution_count: 4,
            cap_half: 1,
            excess_units: 1,
            widening_step: 1,
            widening_scale: 1,
        },
    ];
    for (index, case) in cases.iter().enumerate() {
        check_case(case, &limits, &envelopes)
            .unwrap_or_else(|error| panic!("property body case {index} ({case:?}): {error}"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-N2: concrete real-kernel widening regressions.
// ─────────────────────────────────────────────────────────────────────────────

/// Root cause: `WideningBeadingStrategy::compute` branched on
/// `thickness < optimal_width` alone, so a caller that requested two beads for
/// a region thinner than the optimum got one widened bead instead of the
/// parent's two-bead distribution.
///
/// Canonical `WideningBeadingStrategy::compute` (`WideningBeadingStrategy.cpp`)
/// gates its special branch on `bead_count <= 1 && thickness < getTransitionThickness(1)`
/// and otherwise delegates. Input: distributed parent, `w = 4000` units,
/// `t = 3000` units, requested count 2.
pub fn widening_two_requested_below_optimal_delegates_to_parent() {
    let widening =
        WideningBeadingStrategy::new(Box::new(distributed(4000.0)), 4000.0, 1000.0, 3400.0);
    let parent = distributed(4000.0);
    let expected = parent.compute(3000.0, 2);
    let actual = widening.compute(3000.0, 2);

    assert_eq!(
        actual.bead_widths,
        vec![1500.0, 1500.0],
        "a two-bead request below optimal_width must delegate to the parent (two 1500-unit \
         beads); one widened bead means the removed thickness-only gate is back"
    );
    assert_eq!(
        actual.toolpath_locations,
        vec![750.0, 2250.0],
        "the delegated two-bead centres must be t/4 and 3t/4"
    );
    assert_eq!(
        actual.left_over, 0.0,
        "the delegated two-bead split leaves no surplus"
    );
    assert_eq!(
        actual, expected,
        "the delegated output must be exactly the parent's own compute result"
    );
}

/// Root cause guarded: the single-bead widening gate must not swallow the
/// two-bead endpoint that the real transition caller
/// (`populate_beading_propagation`) requests at a `transition_ratio` vertex.
///
/// Derivation (all in slicer units; `1 unit = 100 nm`):
///
/// * the fixture vertex sits at radius 1500, so `thickness = 2 * 1500 = 3000`;
/// * `create_stack` computes `wall_split_middle_threshold =
///   clamp(2 * min_output_width / (preferred_outer + layer_height * (1 - pi/4)) - 1,
///   0.01, 0.99) = 6800 / 4429.2037 - 1 = 0.53527`, hence
///   `get_transition_thickness(1) = (1 + 0.53527) * 4000 = 6141.1 > 3000`, so the
///   `bead_count = 1` call is the single-bead branch: `max(3000, 3400) = 3400`
///   centred at 1500;
/// * the `bead_count = 2` call delegates: `Redistribute` has no inner region at
///   `bead_count == 2`, so it splits the region into two 1500-unit outer beads
///   at 750 and 2250;
/// * `interpolate(low, 1 - 0.5, high)` keeps the two-bead list (equal
///   thicknesses fall to `high`) and blends index 0 only:
///   `0.5 * 3400 + 0.5 * 1500 = 2450` at `0.5 * 1500 + 0.5 * 750 = 1125`, while
///   index 1 stays 1500 at 2250.
///
/// The expectation is unaffected by the widening gate fix, so this test must
/// pass before and after it; it pins the real caller's two-bead endpoint.
pub fn widening_transition_propagation_preserves_two_bead_endpoint() {
    let params = BeadingFactoryParams {
        print_thin_walls: true,
        max_bead_count: 4,
        optimal_width: 4000.0,
        preferred_bead_width_outer: 4000.0,
        min_input_width: 1000.0,
        min_output_width: 3400.0,
        outer_wall_offset: 0.0,
        ..Default::default()
    };
    let strategy = BeadingStrategyFactory::create_stack(&params);
    assert!(
        strategy.type_chain().contains("Widening"),
        "the fixture must actually carry the Widening layer, got {}",
        strategy.type_chain()
    );

    // A bounded two-vertex / twin-edge graph. Vertex 0 is the noncentral
    // transition node; vertex 1 is a boundary (rib-foot) node with no bead
    // count, so the population pass leaves its side-table slot empty.
    let vertices = vec![
        // exhaustive: STVertex has no Default implementation; all four fields are pinned.
        STVertex {
            position: Vertex { x: 0.0, y: 0.0 },
            distance_to_boundary: 1500.0,
            bead_count: Some(1),
            transition_ratio: 0.5,
        },
        // exhaustive: STVertex has no Default implementation; all four fields are pinned.
        STVertex {
            position: Vertex { x: 0.0, y: 1500.0 },
            distance_to_boundary: 0.0,
            bead_count: None,
            transition_ratio: 0.0,
        },
    ];
    let edges = vec![
        STHalfEdge {
            start_vertex: 0,
            twin: 1,
            ..STHalfEdge::default()
        },
        STHalfEdge {
            start_vertex: 1,
            twin: 0,
            ..STHalfEdge::default()
        },
    ];
    let mut graph = SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: true,
        ..Default::default()
    };

    populate_beading_propagation(&mut graph, strategy.as_ref());

    assert!(
        graph.beading_propagation[1].is_none(),
        "the rib-foot vertex must not receive a beading"
    );
    let beading = graph.beading_propagation[0]
        .as_ref()
        .expect("the transition vertex carries a beading");

    // `interpolate` performs two rounded multiplies and one rounded add per
    // blended entry, plus the `1.0 - transition_ratio` subtraction and the two
    // strategies' own width arithmetic. Budget 12 rounded operations over a
    // monomial magnitude of `|2450| + |1500| + |2250| + |1125|`, rounded up to
    // 12000 units.
    let envelope = Envelope::from_operations(12_000.0, 12).expect("finite transition envelope");

    assert_eq!(
        beading.bead_widths.len(),
        2,
        "the real transition caller must preserve the two-bead endpoint, got {:?}",
        beading.bead_widths
    );
    let expected_widths = [2450.0_f64, 1500.0];
    for (index, (&observed, &expected)) in beading
        .bead_widths
        .iter()
        .zip(expected_widths.iter())
        .enumerate()
    {
        assert!(
            envelope.contains(observed, expected),
            "bead_widths[{index}]: derived {expected}, observed {observed} (envelope {})",
            envelope.tolerance(expected)
        );
    }
    let expected_locations = [1125.0_f64, 2250.0];
    for (index, (&observed, &expected)) in beading
        .toolpath_locations
        .iter()
        .zip(expected_locations.iter())
        .enumerate()
    {
        assert!(
            envelope.contains(observed, expected),
            "toolpath_locations[{index}]: derived {expected}, observed {observed} (envelope {})",
            envelope.tolerance(expected)
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-N2 residual: canonical cap clamp in `LimitedBeadingStrategy`.
// ─────────────────────────────────────────────────────────────────────────────

/// Canonical `LimitedBeadingStrategy::getTransitionThickness` clamps at the
/// cap: below it the parent answers unchanged; at or above it canonical
/// `assert(false)`s and computes `parent.getOptimalThickness(cap + 1) -
/// 0.01 mm`. This port saturates that same value.
///
/// The regression failed before the fix. The pre-fix implementation delegated
/// unconditionally, so `get_transition_thickness(cap)` returned the parent's
/// threshold interpolation instead of canonical's floor. On this fixture
/// (`width = 4_000`, `cap = 4`, distributed parent, `0.5` add-middle
/// threshold) that is `16_000 + 4_000 * 0.5 = 18_000` units, while the
/// canonical floor is `4_000 * 5 - 0.01 * 10_000 = 19_900` units.
pub fn limited_transition_thickness_at_cap_matches_canonical_floor() {
    let width = 4_000.0_f64;
    let cap = 4_usize;
    let parent = distributed(width);
    let limited = LimitedBeadingStrategy::new(Box::new(distributed(width)), cap);

    // Below the cap the decorator must be an exact pass-through.
    for lower in 0..cap {
        assert_eq!(
            limited.get_transition_thickness(lower),
            parent.get_transition_thickness(lower),
            "lower_bead_count {lower} < cap {cap} must delegate to the parent unchanged"
        );
    }

    // Canonical's saturated at-or-above-cap value.
    let canonical_floor = parent.optimal_thickness(cap + 1) - 0.01 * UNITS_PER_MM;
    assert_eq!(
        limited.get_transition_thickness(cap),
        canonical_floor,
        "at the cap the clamp must return parent.optimal_thickness(cap + 1) - 0.01 mm"
    );
    assert_eq!(
        limited.get_transition_thickness(cap + 3),
        canonical_floor,
        "above the cap the port saturates the same clamped value"
    );

    // Record, in the regression itself, the value the pre-fix delegation
    // returned — the assertion above failed with this number.
    let pre_fix = parent.get_transition_thickness(cap);
    assert_eq!(
        pre_fix, 18_000.0,
        "the distributed parent's transition value at the cap is the 0.5-interpolated 18_000"
    );
    assert_ne!(
        pre_fix, canonical_floor,
        "the pre-fix delegation must not equal the canonical floor"
    );
    assert!(
        pre_fix < canonical_floor,
        "the pre-fix delegation returned 18_000, below the canonical floor 19_900"
    );
}
