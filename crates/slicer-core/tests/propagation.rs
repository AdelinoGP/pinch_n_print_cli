//! Bead-count propagation + transition-marking tests for
//! `propagate_beadings_upward`/`propagate_beadings_downward` (T-222, packet
//! 112 Step 3 of the M2 Arachne port).
//!
//! The cases below use source polygons and assert propagation invariants
//! rather than serialized output from one implementation run.
//!
//! # `transition_flags`/`STHalfEdge::transition_mids` (packet 113b onward)
//!
//! This file originally checked two now-removed `STHalfEdge` booleans
//! (`is_transition_middle`/`is_transition_end`) that `propagate_beadings_*`
//! stopped setting once packet 113b introduced the real transition mechanism
//! (`transition_mids`, populated by `generate_transition_mids` — a separate
//! pass this file's fixtures do not call). [`transition_flags`] reads the
//! real field, but none of the three fixtures below currently populate it, so
//! [`assert_transitions_imply_differing_neighbor`] remains dormant (its body
//! never executes past the empty-check) — same as it always has been in this
//! file. Wiring `generate_transition_mids` into these fixtures to make that
//! check non-trivial is left to a follow-on: it surfaces real geometric edge
//! cases (e.g. a leaf/tip vertex with no other central-edge neighbor to
//! cross-check against) that this helper's current design does not yet
//! account for.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features.

#![cfg(feature = "host-algos")]

use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    apply_transitions, assign_bead_counts, filter_central, generate_transition_mids,
    populate_beading_propagation, propagate_beadings_downward, propagate_beadings_upward,
    CentralityParams, EdgeType, RibData, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
    TransitionMiddle,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
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

/// Uniform source geometry: a needle-like isosceles triangle.
fn uniform_square_fixture() -> ExPolygon {
    expoly(vec![p(0.0, 0.0), p(0.1, 0.0), p(0.1, 0.1), p(0.0, 0.1)])
}

/// Parameters for the uniform source geometry.
fn factory_params() -> BeadingFactoryParams {
    BeadingFactoryParams {
        optimal_width: 20.0,
        default_transition_length: 20.0,
        transition_filter_dist: 10.0,
        distribution_count: 1,
        min_input_width: 5.0,
        min_output_width: 20.0,
        outer_wall_offset: 0.0,
        max_bead_count: 10,
        minimum_variable_line_ratio: 0.5,
        print_thin_walls: false,
        preferred_bead_width_outer: 20.0,
        wall_transition_angle: 0.17453292519943295,
        initial_layer_min_bead_width: 20.0,
        ..Default::default()
    }
}

/// Same tightened `CentralityParams` as `tests/centrality.rs`/
/// `tests/bead_count.rs` (a nonzero `min_central_distance` floor so both the
/// wedge and multi-feature fixtures get a genuine central/non-central mix).
///
/// The `transition_filter_dist` is multiplied by a small fraction before
/// passing to `filter_central` so the `dR < dD * sin(angle/2)` predicate
/// dominates for these existing fixtures; otherwise the outer-edge filter
/// would reject the entire tapered wedge (its deepest point is below the
/// unscaled `200.0` threshold).
fn centrality_params() -> CentralityParams {
    CentralityParams::new(200.0, 50.0)
}

const CENTRALITY_TRANSITIONING_ANGLE_RAD: f64 = std::f64::consts::PI;
const OUTER_FILTER_FRACTION: f64 = 0.01;

/// Multi-feature source geometry with a reflex corner.
fn multi_feature_fixture() -> ExPolygon {
    expoly(vec![
        p(0.0, 0.0),
        p(0.0, 0.0),
        p(2.0, 0.0),
        p(2.0, 0.8),
        p(0.8, 0.8),
        p(0.8, 2.0),
        p(0.0, 2.0),
    ])
}

fn varying_wedge_fixture() -> ExPolygon {
    expoly(vec![
        p(0.0, -10.0),
        p(40.0, -1.0),
        p(40.0, 1.0),
        p(0.0, 10.0),
    ])
}

/// Builds a fresh graph for `poly`, runs `filter_central` then
/// `assign_bead_counts` with a freshly-built strategy instance from `params`.
/// Does *not* run `generate_transition_mids`/`apply_transitions` or
/// propagation yet — callers apply
/// `propagate_beadings_upward`/`_downward` themselves so tests can inspect
/// intermediate state.
fn build_filtered_and_assigned_with(
    poly: &ExPolygon,
    params: &BeadingFactoryParams,
) -> SkeletalTrapezoidationGraph {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");

    let mut centrality_params = centrality_params();
    centrality_params.transition_filter_dist *= OUTER_FILTER_FRACTION;
    filter_central(
        &mut graph,
        &centrality_params,
        CENTRALITY_TRANSITIONING_ANGLE_RAD,
    );

    let strategy = BeadingStrategyFactory::create_stack(params);
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("centrality was run, so assign_bead_counts must succeed");

    graph
}

/// Bead count on the `to` vertex of edge `edge_idx`.
fn vertex_bead_count_at_end(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> Option<u32> {
    let edge = graph.edges.get(edge_idx)?;
    let to_v = if edge.twin == NO_INDEX {
        NO_INDEX
    } else {
        graph.edges.get(edge.twin)?.start_vertex
    };
    graph.vertices.get(to_v).and_then(|v| v.bead_count)
}

/// Hand-built graph with one genuine `bead_count == None` gap, to prove
/// `fill_gaps` (exercised indirectly by both `propagate_beadings_upward` and
/// `propagate_beadings_downward`) is not vestigial: in the packet's own
/// pipeline, `assign_bead_counts` already assigns every central edge, so
/// gap-filling is a no-op there (see `propagation.rs`'s module doc comment).
/// A 3-vertex path v0-v1-v2: edge A (v0<->v1) has `bead_count = Some(4)`;
/// edge B (v1<->v2) starts with `bead_count = None` on both its
/// half-edges. `propagate_beadings_upward` must fill both of B's directions
/// from A (the only central neighbor reachable at v1) with `Some(4)`.
/// Same shape as [`gapped_hand_built_graph`] but with a *thickness gradient*: a
/// thin source vertex that owns a bead count feeds a MUCH thicker gap vertex.
/// This is the benchy-hull-spine shape (`D4-INNER-WALL-OVEREXTRUSION`), and the
/// case [`gapped_hand_built_graph`] structurally cannot catch because every one
/// of its vertices shares `distance_to_boundary = 5.0` (with uniform thickness,
/// copying a beading and recomputing it are indistinguishable).
///
/// Scaled to this file's synthetic [`factory_params`] (`optimal_width = 20`
/// units), not to mm: v0/v1 sit at `distance_to_boundary = 30` (thickness 60 =
/// exactly `3 * optimal_width`, so `bead_count = 3` is genuinely optimal there
/// and `compute` yields three 20-wide beads). v2 sits at
/// `distance_to_boundary = 500` (thickness 1000 — the "hull spine" analogue,
/// ~16x too thick for 3 beads) with `bead_count = None` (a real gap / purely
/// propagated node). Recomputing the source's `bead_count = 3` at v2's own
/// thickness would yield `[20, 960, 20]` — the giant centre bead this pins.
fn thickness_gradient_gap_graph() -> SkeletalTrapezoidationGraph {
    fn vertex(x: f64, r: f64, bc: Option<u32>) -> STVertex {
        STVertex {
            position: Vertex { x, y: 0.0 },
            distance_to_boundary: r,
            bead_count: bc,
            transition_ratio: 0.0,
        }
    }
    fn edge(start_vertex: usize, twin: usize, r_min: f64, r_max: f64) -> STHalfEdge {
        STHalfEdge {
            start_vertex,
            twin,
            next: NO_INDEX,
            prev: NO_INDEX,
            r_min,
            r_max,
            central: true,
            is_curved: false,
            rib_twin: None,
            quad_cell: None,
            edge_type: EdgeType::NORMAL,
            transition_mids: Vec::new(),
            transition_ends: Vec::new(),
        }
    }

    // Scaled to `factory_params()`'s synthetic `optimal_width = 20` units.
    const THIN_R: f64 = 30.0; // thickness 60 == 3 * optimal_width
    const THICK_R: f64 = 500.0; // thickness 1000 — the "spine" analogue

    let vertices = vec![
        vertex(0.0, THIN_R, Some(3)),
        vertex(10.0, THIN_R, Some(3)),
        vertex(20.0, THICK_R, None),
    ];
    let edges = vec![
        edge(0, 1, 0.0, THIN_R),
        edge(1, 0, 0.0, THIN_R),
        edge(1, 3, THIN_R, THICK_R), // v1 -> v2: upward into the thick gap
        edge(2, 2, THIN_R, THICK_R),
    ];

    SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: true,
        rib: RibData::default(),
        ..Default::default()
    }
}

/// Regression pin for `D4-INNER-WALL-OVEREXTRUSION`. Unit-independent
/// STRUCTURAL invariant: a beading propagated upward must be the source's
/// beading **verbatim** — never rescaled to the destination's (larger)
/// thickness, which would inflate a middle bead into a physically impossible
/// extrusion.
///
/// Canonical `propagateBeadingsUpward` (`SkeletalTrapezoidation.cpp:1561-1588`)
/// copies the lower node's whole `Beading` and asserts
/// `upper_beading.beading.total_thickness <= to->distance_to_boundary * 2` — a
/// propagated beading is EXPECTED to be thinner than its destination; the
/// surplus region is infill, not extrudate. The pre-fix code instead propagated
/// the scalar `bead_count` and let `populate_beading_propagation` recompute
/// `compute(2 * 9.87mm, 3)` = `[0.4, 18.9, 0.4]` at the destination — a ~19mm
/// "wall" (43x a 0.45mm nozzle). On benchy that produced 3,313 inner-wall moves
/// wider than 3mm (max 19.6mm); Classic produced zero.
#[test]
fn propagation_upward_copies_beading_without_rescaling_to_thicker_node() {
    let mut graph = thickness_gradient_gap_graph();
    let params = factory_params();
    let strategy = BeadingStrategyFactory::create_stack(&params);

    populate_beading_propagation(&mut graph, strategy.as_ref());
    propagate_beadings_upward(&mut graph);

    let thick = graph.beading_propagation[2]
        .as_ref()
        .expect("the thick gap vertex must receive a propagated beading from its thin neighbour");

    // (a) Verbatim copy of the source's beading.
    assert_eq!(
        Some(thick),
        graph.beading_propagation[1].as_ref(),
        "the propagated beading must be the source's verbatim, not recomputed at the \
         destination's thickness"
    );

    // (b) The invariant that actually fails on the D4 defect: no bead may be
    // inflated to swallow the destination's surplus thickness. Canonical keeps
    // every propagated bead at ~optimal width; the surplus becomes infill.
    let widest = thick.bead_widths.iter().cloned().fold(0.0_f64, f64::max);
    assert!(
        widest <= 2.0 * params.optimal_width,
        "propagated beading has a bead of width {widest} units (> 2x optimal_width {}) — the \
         destination's surplus thickness was materialised as an extruded bead instead of infill \
         (D4-INNER-WALL-OVEREXTRUSION)",
        params.optimal_width
    );

    // (c) Canonical's own assert: a propagated beading is thinner than its
    // destination (`SkeletalTrapezoidation.cpp:1587`).
    assert!(
        thick.total_thickness <= 2.0 * graph.vertices[2].distance_to_boundary,
        "propagated beading total_thickness {} must not exceed the destination's own \
         thickness {} (canonical SkeletalTrapezoidation.cpp:1587)",
        thick.total_thickness,
        2.0 * graph.vertices[2].distance_to_boundary
    );

    // (d) Canonical never writes `bead_count` on a purely propagated joint.
    assert_eq!(
        graph.vertices[2].bead_count, None,
        "a purely propagated joint must keep bead_count = None (canonical -1)"
    );
}

fn gapped_hand_built_graph() -> SkeletalTrapezoidationGraph {
    fn vertex(x: f64, y: f64, bc: Option<u32>) -> STVertex {
        STVertex {
            position: Vertex { x, y },
            distance_to_boundary: 5.0,
            bead_count: bc,
            transition_ratio: 0.0,
        }
    }

    fn edge(start_vertex: usize, twin: usize, r_min: f64, r_max: f64) -> STHalfEdge {
        STHalfEdge {
            start_vertex,
            twin,
            next: NO_INDEX,
            prev: NO_INDEX,
            r_min,
            r_max,
            central: true,
            is_curved: false,
            rib_twin: None,
            quad_cell: None,
            edge_type: EdgeType::NORMAL,
            transition_mids: Vec::new(),
            transition_ends: Vec::new(),
        }
    }

    let vertices = vec![
        vertex(0.0, 0.0, Some(4)),
        vertex(10.0, 0.0, Some(4)),
        vertex(20.0, 0.0, None),
    ];

    let edges = vec![
        edge(0, 1, 0.0, 5.0),  // 0: A v0->v1
        edge(1, 0, 0.0, 5.0),  // 1: A v1->v0
        edge(1, 3, 5.0, 10.0), // 2: B v1->v2 (gap)
        edge(2, 2, 5.0, 10.0), // 3: B v2->v1 (gap)
    ];

    SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: true,
        rib: RibData::default(),
        ..Default::default()
    }
}

#[test]
fn transitions_present_where_bead_count_changes() {
    for (name, poly, params, expects_change) in [
        (
            "uniform",
            uniform_square_fixture(),
            BeadingFactoryParams::default(),
            false,
        ),
        (
            "varying",
            varying_wedge_fixture(),
            BeadingFactoryParams::default(),
            true,
        ),
        (
            "multi-feature",
            multi_feature_fixture(),
            BeadingFactoryParams::default(),
            true,
        ),
    ] {
        let mut graph = build_filtered_and_assigned_with(&poly, &params);
        let strategy = BeadingStrategyFactory::create_stack(&params);
        generate_transition_mids(&mut graph, strategy.as_ref());

        let mut changed_edges = 0;
        for (idx, edge) in graph.edges.iter().enumerate() {
            if !edge.central {
                continue;
            }
            let end = vertex_bead_count_at_end(&graph, idx);
            let start = graph
                .vertices
                .get(edge.start_vertex)
                .and_then(|v| v.bead_count);
            if let (Some(start), Some(end)) = (start, end) {
                if start != end {
                    changed_edges += 1;
                    assert!(
                        !edge.transition_mids.is_empty()
                            || !edge.transition_ends.is_empty()
                            || graph.edges.get(edge.twin).is_some_and(|twin| {
                                !twin.transition_mids.is_empty() || !twin.transition_ends.is_empty()
                            }),
                        "{name}: central edge {idx} changes bead count from {start} to {end} \
                         without a transition marker"
                    );
                }
            }
        }

        assert_eq!(
            changed_edges > 0,
            expects_change,
            "{name}: source geometry did not match its declared bead-count class"
        );

        propagate_beadings_upward(&mut graph);
        propagate_beadings_downward(&mut graph);
        assert!(
            !graph.edges.is_empty(),
            "{name}: propagation must retain a non-empty source-derived graph"
        );
    }
}

/// Supplementary (not part of the required three-fixture AC): directly
/// exercises `fill_gaps` (indirectly, via `propagate_beadings_upward`) on a
/// hand-built graph with a genuine `bead_count == None` gap, proving
/// gap-filling is not vestigial dead code — see `gapped_hand_built_graph`'s
/// doc comment for why the packet's own real pipeline never exercises this
/// path (every central edge already has a `bead_count` by the time
/// propagation runs).
#[test]
fn propagation_fills_gap_from_central_neighbor() {
    let mut graph = gapped_hand_built_graph();
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());

    assert_eq!(
        graph.vertices[2].bead_count, None,
        "precondition: vertex 2 (v2) must start as a genuine gap"
    );

    // Canonical pass order (`SkeletalTrapezoidation.cpp:1488-1508`): each node
    // with its OWN bead count gets a beading first; only then is that beading
    // propagated to nodes that have none.
    populate_beading_propagation(&mut graph, strategy.as_ref());
    assert!(
        graph.beading_propagation[2].is_none(),
        "precondition: the gap vertex must have no beading of its own before propagation"
    );

    propagate_beadings_upward(&mut graph);

    // Canonical `propagateBeadingsUpward` (`SkeletalTrapezoidation.cpp:1561-1588`)
    // fills the gap by copying the neighbour's BEADING and deliberately leaves
    // the joint's `bead_count` at its default (`-1`, `None` here) — it only calls
    // `setBeading`, never assigns `to->bead_count`. This test previously asserted
    // the opposite (`bead_count == Some(4)`), pinning a real defect: propagating
    // the scalar and letting `populate_beading_propagation` recompute
    // `compute(2 * to.distance_to_boundary, bead_count)` at the DESTINATION's
    // thickness turns a thin node's bead count into a giant centre bead on a
    // thicker node (benchy hull spine: `compute(19.7mm, 3)` = `[0.4, 18.9, 0.4]`,
    // a ~19mm extruded "wall" — `D4-INNER-WALL-OVEREXTRUSION`). This fixture never
    // caught it because every vertex here shares `distance_to_boundary = 5.0`, so
    // copy-vs-recompute are indistinguishable; see
    // `propagation_upward_copies_beading_without_rescaling_to_thicker_node` for the
    // differing-thickness case that does catch it.
    assert_eq!(
        graph.vertices[2].bead_count, None,
        "vertex 2 (v2): canonical propagation must NOT write `bead_count` on a purely \
         propagated joint — it stays -1/None and only receives a beading"
    );
    let filled = graph.beading_propagation[2].as_ref().expect(
        "vertex 2 (v2): gap must be filled from its only central neighbor at v1 \
                 (edge 0/1, bead_count=4) by copying v1's beading into the side table",
    );
    assert_eq!(
        Some(filled),
        graph.beading_propagation[1].as_ref(),
        "vertex 2 (v2): the propagated beading must be a verbatim copy of v1's"
    );

    // Filling a gap so both sides now agree (4 == 4) must not spuriously
    // populate transition_mids — `propagate_beadings_upward` only ever
    // writes the beading side table, never `transition_mids` on edges
    // (that's `generate_transition_mids`'s job, not called in this
    // gap-filling-only test).
    assert!(
        graph.edges[0].transition_mids.is_empty() && graph.edges[2].transition_mids.is_empty(),
        "gap-filled graph must have zero transition_mids entries (propagate_beadings_upward \
         does not populate them); got edge0={:?}, edge2={:?}",
        graph.edges[0].transition_mids,
        graph.edges[2].transition_mids
    );
}

/// Hand-built graph exercising `insert_node`'s DCEL rewiring under the packet
/// 113c interleaved-rib topology: a single central edge `E0` (v0 -> v1, along
/// y=0 from x=0 to x=100) carries *two* `transition_mids` (so `apply_transitions`
/// performs two same-edge splits on `E0` — the exact repeated-same-edge-split
/// shape from `D-112-MMU-TOPOLOGY`'s 6th-pass "busy-hub" bug history), and `v1`
/// is immediately followed by a rib pair (`E2` forth / `E3` back, `EdgeType::
/// EXTRA_VD`) built with the *exact* wiring `SkeletalTrapezoidationGraph::
/// make_rib` produces: `E0.next = E2`, `E2.prev = E0`, `E2.twin = E3`,
/// `E3.twin = E2`, and `E3.prev = NO_INDEX` (never assigned — the domain/quad-
/// start marker). `E1` is `E0`'s cross-cell mirror twin (`E0.twin = E1`,
/// `E1.twin = E0`), carrying no `transition_mids` of its own so its own splits
/// come *only* from `apply_transitions`'s twin-mirroring step.
///
/// Vertices: v0 (x=0, r=0, bead_count=2), v1 (x=100, r=10, bead_count=5,
/// the rib's spine anchor), v2 (x=100,y=-10, rib foot, r=0).
fn rib_adjacent_two_split_graph() -> SkeletalTrapezoidationGraph {
    fn vertex(x: f64, y: f64, r: f64, bc: Option<u32>) -> STVertex {
        STVertex {
            position: Vertex { x, y },
            distance_to_boundary: r,
            bead_count: bc,
            transition_ratio: 0.0,
        }
    }

    let vertices = vec![
        vertex(0.0, 0.0, 0.0, Some(2)),    // 0: v0 (edge start)
        vertex(100.0, 0.0, 10.0, Some(5)), // 1: v1 (edge end / rib spine anchor)
        vertex(100.0, -10.0, 0.0, None),   // 2: v2 (rib foot)
    ];

    fn edge(
        start_vertex: usize,
        twin: usize,
        next: usize,
        prev: usize,
        central: bool,
        edge_type: EdgeType,
        transition_mids: Vec<TransitionMiddle>,
    ) -> STHalfEdge {
        STHalfEdge {
            start_vertex,
            twin,
            next,
            prev,
            r_min: 0.0,
            r_max: 10.0,
            central,
            is_curved: false,
            rib_twin: None,
            quad_cell: None,
            edge_type,
            transition_mids,
            transition_ends: Vec::new(),
        }
    }

    // E0: orig (v0 -> v1), central, carrying the two transition ends that
    // drive the same-edge repeated splits under test. `next = 2` (the rib's
    // forth edge), matching real `make_rib` wiring exactly.
    let e0 = edge(
        0,
        1,
        2,
        NO_INDEX,
        true,
        EdgeType::NORMAL,
        vec![
            TransitionMiddle {
                pos: 0.3,
                lower_bead_count: 2,
                mid_r: 3.0,
            },
            TransitionMiddle {
                pos: 0.7,
                lower_bead_count: 3,
                mid_r: 7.0,
            },
        ],
    );
    // E1: T, E0's cross-cell mirror twin (v1 -> v0 in T's own frame). No
    // trailing rib of its own (`next = NO_INDEX`) — a distinct, valid chain
    // shape (this cell's final closing edge never gets a rib either).
    let e1 = edge(1, 0, NO_INDEX, NO_INDEX, true, EdgeType::NORMAL, Vec::new());
    // E2: rib forth edge (v1 -> rib foot), non-central, dead end (`next =
    // NO_INDEX`), `prev = 0` matching `make_rib`'s `edges[forth].prev = pe`.
    let e2 = edge(1, 3, NO_INDEX, 0, false, EdgeType::EXTRA_VD, Vec::new());
    // E3: rib back edge (rib foot -> v1), non-central, `prev = NO_INDEX`
    // permanently (the domain/quad-start marker — never assigned by
    // `make_rib`), `next = NO_INDEX` (no further spine edge follows in this
    // minimal test cell).
    let e3 = edge(
        2,
        2,
        NO_INDEX,
        NO_INDEX,
        false,
        EdgeType::EXTRA_VD,
        Vec::new(),
    );

    SkeletalTrapezoidationGraph {
        vertices,
        edges: vec![e0, e1, e2, e3],
        centrality_filtered: true,
        rib: RibData::default(),
        ..Default::default()
    }
}

/// AC-6 dedicated regression test (packet 113c Step 6, rewritten for the
/// 113d parity fix). Verifies `insert_node`/`apply_transitions`'s
/// same-edge-repeated-split correctness against the faithful OrcaSlicer
/// `insertNode`+`insertRib` semantics: each split creates 1 shared
/// mid_node (carrying `bead_count` + `distance_to_boundary = mid_r`) +
/// 2 boundary foot nodes (`distance_to_boundary = 0`), splits BOTH the
/// edge and its twin atomically at the same physical position, and
/// cross-patches the twins.
///
/// This test supersedes the original 113c version which asserted the OLD
/// broken behavior (2 fragments per side via independent twin-side
/// splitting, `transition_ratio = 0.5`, edge count = 8) that the Arachne
/// parity audit (findings F1, F2, F6) identified as unfaithful to
/// OrcaSlicer. See `target/arachne_parity_audit_*.md`.
#[test]
fn same_edge_splits_near_rib_insertion() {
    let mut graph = rib_adjacent_two_split_graph();

    let n_verts_before = graph.vertices.len();
    let n_edges_before = graph.edges.len();

    apply_transitions(&mut graph);

    // (a) Two splits on E0 (at pos=0.3 and pos=0.7) must each create:
    //   - 1 shared mid_node (carrying bead_count, distance_to_boundary = mid_r)
    //   - 2 boundary foot nodes (distance_to_boundary = 0, bead_count = None)
    // So 2 splits x 3 new vertices = 6 new vertices total.
    let n_new_verts = graph.vertices.len() - n_verts_before;
    assert_eq!(
        n_new_verts, 6,
        "expected 6 new vertices (2 splits x (1 mid_node + 2 foot nodes));          got {n_new_verts}"
    );

    // (b) Each split creates 6 new edges (2 NORMAL "second" fragments +
    // 4 EXTRA_VD rib edges; the original edge and twin are repurposed, not
    // newly created). 2 splits x 6 = 12 new edges.
    let n_new_edges = graph.edges.len() - n_edges_before;
    assert_eq!(
        n_new_edges, 12,
        "expected 12 new edges (2 splits x (2 NORMAL seconds + 4 EXTRA_VD \
         rib pairs)); got {n_new_edges}"
    );

    // (c) The shared mid_nodes are at absolute positions x=30 (bc=2,
    // mid_r=3.0) and x=70 (bc=3, mid_r=7.0), matching the transition_mids.
    // The 2 splits land at the geometrically correct absolute positions
    // (NOT the old-bug x~=17.14 from ascending-order rescale against the
    // wrong span).
    let new_verts: Vec<&STVertex> = graph.vertices[n_verts_before..].iter().collect();
    let mid_nodes: Vec<&STVertex> = new_verts
        .iter()
        .copied()
        .filter(|v| v.bead_count.is_some())
        .collect();
    assert_eq!(
        mid_nodes.len(),
        2,
        "expected exactly 2 shared mid_nodes carrying bead_count; got {}",
        mid_nodes.len()
    );
    let xs: Vec<f64> = mid_nodes.iter().map(|v| v.position.x).collect();
    let bcs: Vec<Option<u32>> = mid_nodes.iter().map(|v| v.bead_count).collect();
    let rs: Vec<f64> = mid_nodes.iter().map(|v| v.distance_to_boundary).collect();
    // Sort by x to get a deterministic order regardless of internal
    // insertion sequence.
    let mut sorted: Vec<(f64, Option<u32>, f64)> = xs
        .into_iter()
        .zip(bcs)
        .zip(rs)
        .map(|((x, bc), r)| (x, bc, r))
        .collect();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    assert!(
        (sorted[0].0 - 30.0).abs() < 1e-6,
        "first mid_node (pos=0.3 transition) must be at absolute x=30, got          {} - the old ascending-order rescale bug would have produced          x~=17.14 here",
        sorted[0].0
    );
    assert_eq!(
        sorted[0].1,
        Some(2),
        "first mid_node bead_count must be 2 (lower_bead_count), got {:?}",
        sorted[0].1
    );
    assert!(
        (sorted[0].2 - 3.0).abs() < 1e-6,
        "first mid_node distance_to_boundary must be mid_r=3.0, got {}",
        sorted[0].2
    );
    assert!(
        (sorted[1].0 - 70.0).abs() < 1e-6,
        "second mid_node (pos=0.7 transition) must be at absolute x=70,          got {} - the old ascending-order rescale bug produced x~=17.14          instead, closer to v0 than the pos=0.3 split - a non-monotonic          inversion",
        sorted[1].0
    );
    assert_eq!(
        sorted[1].1,
        Some(3),
        "second mid_node bead_count must be 3 (lower_bead_count), got {:?}",
        sorted[1].1
    );
    assert!(
        (sorted[1].2 - 7.0).abs() < 1e-6,
        "second mid_node distance_to_boundary must be mid_r=7.0, got {}",
        sorted[1].2
    );

    // (d) The 4 boundary foot nodes carry distance_to_boundary == 0.0 and
    // bead_count == None (OrcaSlicer's `source_node`).
    let foot_nodes: Vec<&STVertex> = new_verts
        .iter()
        .copied()
        .filter(|v| v.bead_count.is_none())
        .collect();
    assert_eq!(foot_nodes.len(), 4, "expected 4 boundary foot nodes");
    for (i, foot) in foot_nodes.iter().enumerate() {
        assert_eq!(
            foot.distance_to_boundary, 0.0,
            "foot node {i} must have distance_to_boundary == 0.0, got {}",
            foot.distance_to_boundary
        );
    }

    // (e) Atomicity (F1+F6): the twin (E1) is NOT independently split.
    // OrcaSlicer's `insertNode` handles both sides in one atomic call, so
    // the twin's fragments connect to the SAME shared mid_nodes. We verify
    // by checking that E0.twin and E1.twin both resolve to a shared
    // mid_node (a vertex carrying a bead_count), NOT the original v0/v1
    // endpoints and NOT a boundary foot node (bead_count == None).
    // E0.twin resolves to the first split's mid_node (x=30); E1.twin
    // resolves to a mid_node too (x=30 or x=70 depending on the chaining
    // order — both are valid shared mid_nodes).
    let e0_twin_resolved = graph
        .vertices
        .get(graph.edges[graph.edges[0].twin].start_vertex);
    assert!(
        e0_twin_resolved.is_some_and(|v| v.bead_count.is_some()),
        "E0.twin must resolve to a shared mid_node (carrying bead_count), \
         proving the twin side was split atomically. Got {:?}",
        e0_twin_resolved.map(|v| (v.position.x, v.bead_count))
    );
    assert!(
        e0_twin_resolved.is_some_and(|v| (v.position.x - 30.0).abs() < 1e-6),
        "E0.twin must resolve to the first split's shared mid_node (x=30). \
         Got x={:?}",
        e0_twin_resolved.map(|v| v.position.x)
    );
    let e1_twin_resolved = graph
        .vertices
        .get(graph.edges[graph.edges[1].twin].start_vertex);
    assert!(
        e1_twin_resolved.is_some_and(|v| v.bead_count.is_some()),
        "E1.twin must resolve to a shared mid_node (carrying bead_count), \
         proving the cross-twin patching links both sides to the same shared \
         node. Got {:?}",
        e1_twin_resolved.map(|v| (v.position.x, v.bead_count))
    );
    let e1_twin_x = e1_twin_resolved.map(|v| v.position.x).unwrap_or(f64::NAN);
    assert!(
        (e1_twin_x - 30.0).abs() < 1e-6 || (e1_twin_x - 70.0).abs() < 1e-6,
        "E1.twin must resolve to one of the shared mid_nodes (x=30 or \
         x=70), proving the twin side was split atomically at the same \
         physical positions as E0. Got x={e1_twin_x}"
    );

    // (f) The original rib (E2 forth / E3 back) must be untouched by
    // `insert_node`'s splits on the central edge feeding into it.
    let forth_idx = 2usize; // E2 (rib forth) keeps its original index.
    let forth = &graph.edges[forth_idx];
    assert_eq!(
        forth.edge_type,
        EdgeType::EXTRA_VD,
        "E2 must remain a rib (EXTRA_VD)"
    );
    let back_idx = forth.twin;
    let back = &graph.edges[back_idx];
    assert_eq!(
        back.prev,
        NO_INDEX,
        "original rib back_edge.prev must remain NO_INDEX (the domain/quad-start          marker) - must not be corrupted by insert_node's rewiring"
    );
    assert_eq!(
        back.twin, forth_idx,
        "rib twin pairing must remain consistent"
    );
}
