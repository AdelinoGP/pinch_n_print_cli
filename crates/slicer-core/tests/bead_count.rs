//! Bead-count assignment tests for `assign_bead_counts` (T-221, packet 112
//! Step 2 of the M2 Arachne port).
//!
//! # Self-captured regression baseline — NOT an OrcaSlicer golden
//!
//! Packet 112 has no OrcaSlicer oracle for this step (see
//! `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs`'s
//! module-level doc comment). `tests/fixtures/arachne/bead_count_tapered_wedge.json`
//! is a **self-captured regression baseline**: on first run,
//! `bead_count_tapered_wedge` writes this implementation's own per-vertex
//! `bead_count` output to disk; on every subsequent run, it compares against
//! the committed file and fails on any drift. This locks in *this*
//! implementation's behavior for regression purposes only — it is not, and
//! must never be described as, independently-derived OrcaSlicer ground truth.
//! The real correctness signal is the invariant assertions
//! (central-adjacent vertex ⇔ `Some`, otherwise `None`, bounds,
//! determinism) documented inline below.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features.

#![cfg(feature = "host-algos")]

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    assign_bead_counts, filter_central, BeadCountError, CentralityParams,
    SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
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
    expoly(vec![p(0, 0), p(10_000, -100), p(10_000, 100)])
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
        max_bead_count: 9,
        minimum_variable_line_ratio: 0.5,
        print_thin_walls: false,
        preferred_bead_width_outer: 20.0,
        wall_transition_angle: 0.17453292519943295,
        initial_layer_min_bead_width: 20.0,
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct BeadCountFixture {
    /// Explicit disclosure: this is a self-captured regression baseline
    /// (this implementation's own output), not an OrcaSlicer golden — see
    /// this file's module-level doc comment.
    provenance: String,
    /// Renamed from `edge_count` in Step 3; kept as `edge_count` for JSON
    /// backward compatibility with the fixture file.
    edge_count: usize,
    /// `bead_count` per vertex index, in `graph.vertices` order.
    bead_counts: Vec<Option<u32>>,
}

const PROVENANCE: &str = "Self-captured regression baseline: serialized from this crate's own \
     assign_bead_counts implementation (packet 112 Step 2 / T-221). NOT derived from, and not a \
     substitute for, OrcaSlicer ground truth — no OrcaSlicer oracle exists for this step (see \
     bead_count.rs's module-level doc comment). Locks in current behavior for regression \
     purposes only.";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/arachne")
        .join("bead_count_tapered_wedge.json")
}

/// Writes `fixture` to disk if absent (first run seeds the baseline);
/// otherwise reads the committed baseline and asserts it matches `fixture`
/// exactly (regression lock).
fn write_or_compare_baseline(fixture: &BeadCountFixture) {
    let path = fixture_path();
    match fs::read_to_string(&path) {
        Ok(existing) => {
            let baseline: BeadCountFixture = serde_json::from_str(&existing).unwrap_or_else(|e| {
                panic!(
                    "{}: failed to parse committed baseline: {e}",
                    path.display()
                )
            });
            assert_eq!(
                &baseline,
                fixture,
                "{}: bead counts drifted from the committed self-captured baseline. If this \
                 drift is an intentional behavior change, delete the file and rerun to re-seed \
                 it (after confirming the new invariants still hold).",
                path.display()
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|e| {
                    panic!("{}: failed to create fixtures dir: {e}", parent.display())
                });
            }
            let json = serde_json::to_string_pretty(fixture)
                .expect("BeadCountFixture serialization is infallible");
            fs::write(&path, json).unwrap_or_else(|e| {
                panic!("{}: failed to write new baseline: {e}", path.display())
            });
        }
        Err(e) => panic!("{}: failed to read baseline: {e}", path.display()),
    }
}

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

#[test]
fn bead_count_tapered_wedge() {
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

    // --- Invariant: every assigned bead count is within [0, max_bead_count] ---
    for (i, bead_count) in bead_counts_a.iter().enumerate() {
        if let Some(n) = bead_count {
            assert!(
                *n <= max_bead_count,
                "vertex {i}: bead_count {n} exceeds max_bead_count {max_bead_count}"
            );
        }
    }

    write_or_compare_baseline(&BeadCountFixture {
        provenance: PROVENANCE.to_string(),
        edge_count: bead_counts_a.len(),
        bead_counts: bead_counts_a,
    });
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
