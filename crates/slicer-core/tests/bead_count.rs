//! Bead-count assignment tests for `assign_bead_counts` (T-221, packet 112
//! Step 2 of the M2 Arachne port).
//!
//! The cases below use source polygons and assert bead-count invariants rather
//! than serialized output from one implementation run.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features.

#![cfg(feature = "host-algos")]

use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    assign_bead_counts, filter_central, BeadCountError, CentralityParams,
    SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon};

fn p(x_mm: f32, y_mm: f32) -> Point2 {
    Point2::from_mm(x_mm, y_mm)
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// Tapered-wedge fixture: the same needle-like isoceles triangle used by
/// `tests/centrality.rs`'s `wedge_fixture` and `tests/skt_graph_golden.rs`'s
/// wedge case (acute apex at the origin, blunt end at x = 10000) — its
/// medial axis has a large depth swing (`r` grows from ~0 at the apex to
/// ~99 at the blunt end), so it exercises both a range of `r_avg` values
/// (varied bead counts) and, under the same tightened `CentralityParams` as
/// `centrality.rs`, a genuine mix of central/non-central edges.
fn tapered_wedge_fixture() -> ExPolygon {
    expoly(vec![p(0.0, 0.0), p(1.0, -0.01), p(1.0, 0.01)])
}

/// Beading-strategy factory params scaled to the tapered wedge's `r` range
/// (~0 to ~99 units), so `optimal_bead_count` actually varies across edges
/// instead of degenerating to a single value. Not derived from any
/// production default — a reasonable, self-consistent instance built for
/// this fixture's scale.
fn factory_params() -> BeadingFactoryParams {
    BeadingFactoryParams {
        optimal_width: 20.0,
        default_transition_length: 20.0,
        transition_filter_dist: 10.0,
        distribution_count: 1,
        min_input_width: 5.0,
        min_output_width: 20.0,
        outer_wall_offset: 0.0,
        // OrcaSlicer always derives `max_bead_count = 2 * inset_count`
        // (`WallToolPaths.cpp:525`), which is ALWAYS EVEN; `LimitedBeadingStrategy`'s
        // ctor warns on an odd count (`LimitedBeadingStrategy.cpp:36-40`) because its
        // odd-`max_bead_count` `compute` branch (`:73`, ported at
        // `crates/slicer-core/src/beading/limited.rs`) parks the entire capped surplus
        // into one physically-impossible wide centre bead instead of splitting it
        // symmetrically. This fixture previously hardcoded odd `9`; `10` is the
        // nearest even value (`inset_count = 5`) and keeps the wedge's ~0..99-unit `r`
        // range exercising the same variety of uncapped bead counts the original `9`
        // was chosen for, without ever taking the odd over-cap branch.
        max_bead_count: 10,
        minimum_variable_line_ratio: 0.5,
        print_thin_walls: false,
        preferred_bead_width_outer: 20.0,
        wall_transition_angle: 0.17453292519943295,
        initial_layer_min_bead_width: 20.0,
        ..Default::default()
    }
}

/// Same tightened `CentralityParams` as `centrality.rs`'s wedge fixture:
/// a nonzero `min_central_distance` floor so the wedge's shallow
/// boundary-adjacent structure is rejected while its genuine medial-axis
/// hub stays central, giving this test a real mix of both.
///
/// The `transition_filter_dist` is multiplied by a small fraction before
/// passing to `filter_central` so the `dR < dD * sin(angle/2)` predicate
/// dominates for this existing fixture; otherwise the outer-edge filter
/// would reject the entire tapered wedge (its deepest point is below the
/// unscaled `200.0` threshold).
fn centrality_params() -> CentralityParams {
    CentralityParams::new(200.0, 50.0)
}

const CENTRALITY_TRANSITIONING_ANGLE_RAD: f64 = 0.17453292519943295; // 10°
const OUTER_FILTER_FRACTION: f64 = 0.01;
/// Builds a fresh graph for `poly`, runs `filter_central` then
/// `assign_bead_counts` with a freshly-built strategy instance, and returns
/// the per-vertex `bead_count` markers alongside the `central` markers used
/// to check that every vertex touched by a central edge carries a `Some(_).
fn build_and_assign(poly: &ExPolygon) -> (Vec<bool>, Vec<Option<u32>>) {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");

    let mut centrality_params = centrality_params();
    centrality_params.transition_filter_dist *= OUTER_FILTER_FRACTION;
    filter_central(
        &mut graph,
        &centrality_params,
        CENTRALITY_TRANSITIONING_ANGLE_RAD,
    );

    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("centrality was run, so assign_bead_counts must succeed");

    let central_vertex: Vec<bool> = graph
        .vertices
        .iter()
        .enumerate()
        .map(|(v_idx, _)| {
            graph.edges.iter().any(|e| {
                e.central
                    && (e.start_vertex == v_idx
                        || (e.twin != NO_INDEX
                            && graph
                                .edges
                                .get(e.twin)
                                .map(|twin| twin.start_vertex == v_idx)
                                .unwrap_or(false)))
            })
        })
        .collect();
    let bead_counts: Vec<Option<u32>> = graph.vertices.iter().map(|v| v.bead_count).collect();
    (central_vertex, bead_counts)
}

/// Same graph/strategy build as [`build_and_assign`], but also returns each
/// vertex's `position.x` and `distance_to_boundary` (`r`) alongside its
/// `bead_count` — needed by the structural invariants below (ADR-0042),
/// which check *where* a bead count sits and *how wide the beading strategy
/// would actually make it*, not just that a count was assigned.
fn build_assign_and_measure(poly: &ExPolygon) -> Vec<(f64, f64, Option<u32>)> {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");

    let mut centrality_params = centrality_params();
    centrality_params.transition_filter_dist *= OUTER_FILTER_FRACTION;
    filter_central(
        &mut graph,
        &centrality_params,
        CENTRALITY_TRANSITIONING_ANGLE_RAD,
    );

    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("centrality was run, so assign_bead_counts must succeed");

    graph
        .vertices
        .iter()
        .map(|v| (v.position.x, v.distance_to_boundary, v.bead_count))
        .collect()
}

#[test]
fn bead_count_sequence_is_monotonic_within_transition_bounds() {
    let wedge = tapered_wedge_fixture();
    let params = factory_params();
    let max_bead_count = params.max_bead_count as u32;

    // --- Run 1 ---
    let (central_a, bead_counts_a) = build_and_assign(&wedge);

    // --- Run 2 (independent graph build, for the determinism invariant) ---
    let (central_b, bead_counts_b) = build_and_assign(&wedge);

    assert_eq!(
        central_a, central_b,
        "centrality must be deterministic across independent builds of the same input"
    );
    assert_eq!(
        bead_counts_a, bead_counts_b,
        "assign_bead_counts must be deterministic: two independent builds of the identical \
         input polygon (and identical strategy params) produced different bead counts"
    );

    // Sanity: the tightened centrality params actually discriminate on this
    // fixture (mirrors centrality.rs's own wedge assertions) — otherwise the
    // central ⇔ Some / non-central ⇔ None invariant below would be checking
    // only one side.
    assert!(
        central_a.iter().any(|&c| c),
        "tapered wedge: expected at least one central-adjacent vertex, got none: {central_a:?}"
    );
    assert!(
        central_a.iter().any(|&c| !c),
        "tapered wedge: expected at least one non-central vertex, got none: {central_a:?}"
    );

    // --- Invariant: central-adjacent vertex ⇔ bead_count is Some, otherwise None ---
    for (i, (&is_central, &bead_count)) in central_a.iter().zip(bead_counts_a.iter()).enumerate() {
        if is_central {
            assert!(
                bead_count.is_some(),
                "vertex {i}: central-adjacent vertex must have bead_count == Some(_), got None"
            );
        } else {
            assert!(
                bead_count.is_none(),
                "vertex {i}: non-central vertex must have bead_count == None, got {bead_count:?}"
            );
        }
    }

    // --- Invariant: every assigned bead count is within [0, max_bead_count + 1] ---
    // The +1 accommodates OrcaSlicer's `LimitedBeadingStrategy::getOptimalBeadCount`
    // cap, which returns `max_bead_count + 1` as the "capped" signal when
    // the parent's uncapped count exceeds the cap (D-105 faithful port).
    // `LimitedBeadingStrategy::compute`'s over-cap branch is then responsible
    // for mapping that +1 back to a beading with `max_bead_count` real beads
    // (plus 2 zero-width sentinels at the cap boundary).
    for (i, bead_count) in bead_counts_a.iter().enumerate() {
        if let Some(n) = bead_count {
            assert!(
                *n <= max_bead_count + 1,
                "vertex {i}: bead_count {n} exceeds max_bead_count + 1 ({})",
                max_bead_count + 1
            );
        }
    }

    // These structural invariants are unit-independent and cover the tapered
    // source geometry's centrality, width, and bead-count sequence.
    let measured = build_assign_and_measure(&wedge);
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    let optimal_width = factory_params().optimal_width;

    // Non-vacuity guard: both invariants below are `for` loops, so they pass
    // trivially on an empty measurement set. Without this, a regression that
    // stopped producing central vertices entirely (i.e. the D5 dropout itself)
    // would turn both invariants green rather than red — the precise "green
    // means unchanged, not correct" failure ADR-0042 exists to prevent.
    assert!(
        !measured.is_empty(),
        "tapered wedge: expected at least one measured vertex, got none — the invariants \
         below would pass vacuously"
    );

    // --- Invariant: no bead the strategy would actually cut is wider than
    // ~2x optimal_width. Directly exercises `BeadingStrategy::compute` at
    // each central vertex's own `(2 * distance_to_boundary, bead_count)` —
    // the exact call shape that produced the D4 "surplus dumped into one
    // giant centre bead" defect when `max_bead_count` was odd
    // (`LimitedBeadingStrategy.cpp:73`; see this crate's
    // `beading/limited.rs`). A regression that reintroduces an odd
    // `max_bead_count`, or otherwise breaks bead redistribution, fails this
    // assertion regardless of what the self-captured snapshot says.
    for &(x, r, bead_count) in &measured {
        if let Some(n) = bead_count {
            let beading = strategy.compute(2.0 * r, n as usize);
            for (bead_idx, &w) in beading.bead_widths.iter().enumerate() {
                assert!(
                    w <= 2.0 * optimal_width + 1e-6,
                    "vertex at x={x}: bead {bead_idx} width {w} exceeds 2x optimal_width \
                     ({}) -- a physically implausible bead, the D4 defect class",
                    2.0 * optimal_width
                );
            }
        }
    }

    // --- Invariant: among central (bead_count == Some) vertices, bead count
    // must not decrease moving from the wedge's acute apex (x = 0, thin) to
    // its blunt end (x = 10_000, thick) -- a tapering wedge that gets thicker
    // must never be assigned FEWER walls. This is exactly the D5 failure
    // shape (a thick region silently dropped to bead_count None/0) restated
    // as a monotonicity property instead of an exact snapshot.
    let mut central_by_x: Vec<(f64, u32)> = measured
        .iter()
        .filter_map(|&(x, _r, bead_count)| bead_count.map(|n| (x, n)))
        .collect();
    central_by_x.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite x coordinates"));
    // Non-vacuity guard: `windows(2)` yields nothing for fewer than 2 elements,
    // so the monotonicity check below would pass trivially on a wedge that had
    // lost its central vertices — exactly the D5 dropout this invariant is
    // meant to catch.
    assert!(
        central_by_x.len() >= 2,
        "tapered wedge: expected at least 2 central (bead_count = Some) vertices to compare, \
         got {} — the monotonicity invariant below would pass vacuously",
        central_by_x.len()
    );
    for pair in central_by_x.windows(2) {
        let (prev_x, prev_n) = pair[0];
        let (next_x, next_n) = pair[1];
        assert!(
            next_n >= prev_n,
            "bead count must not decrease toward the wedge's thick end: x={prev_x} had \
             bead_count={prev_n}, but the next central vertex at x={next_x} (further from \
             the apex) had a lower bead_count={next_n}"
        );
    }
}

/// AC-N1: `assign_bead_counts` must refuse to run on a graph that has never
/// had `filter_central` applied to it (every edge's `central` defaults to
/// `false`, which is indistinguishable from "genuinely no central edges"
/// without the `centrality_filtered` flag — see
/// `SkeletalTrapezoidationGraph::centrality_filtered`'s doc comment).
#[test]
fn bead_count_requires_centrality() {
    let wedge = tapered_wedge_fixture();
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(&[wedge])
        .expect("fixture polygon must build a valid SKT graph");

    assert!(
        !graph.centrality_filtered,
        "freshly-built graph must have centrality_filtered == false"
    );

    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    let result = assign_bead_counts(&mut graph, strategy.as_ref());

    assert!(
        matches!(result, Err(BeadCountError::CentralityNotRun)),
        "assign_bead_counts on a graph that never ran filter_central must return \
         Err(BeadCountError::CentralityNotRun), got {result:?}"
    );
}
