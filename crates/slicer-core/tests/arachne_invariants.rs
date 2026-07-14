//! Faithfulness invariants for the arachne graph construction /
//! `connectJunctions` adaptation, plus a `test_voronoi.cpp` degenerate-input
//! triage note. See doc comment below (only visible when built with the
//! `host-algos` feature) for full detail.
#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

//! Packet 113c Step 8 (AC-8): faithfulness invariants for the graph
//! construction / `connectJunctions` adaptation delivered by Steps 3-7, plus
//! a `test_voronoi.cpp` degenerate-input triage note.
//!
//! # Part 1 -- invariant suite
//!
//! These are properties that hold **by construction** for any well-formed
//! input (not snapshot fixtures pinned to one geometry's exact numeric
//! output):
//!
//! 1. [`outer_wall_is_closed_ring_for_simple_polygons`] -- the outer wall
//!    (`inset_idx == 0`) of a plain closed polygon's toolpath output is a
//!    closed ring.
//! 2. [`quad_chains_span_two_or_three_edges`] -- every domain-start edge
//!    (`.prev == NO_INDEX`) begins a "quad" (the walk-`.next`-to-a-dead-end
//!    run `connectJunctions`/`find_quad` operates on) of exactly 2 or 3
//!    edges, never 1, never >=4 -- see `graph.rs`'s own module doc comment
//!    ("makeRib inserts a rib pair after every transferred edge except each
//!    cell's final closing edge") for why this is the guaranteed shape:
//!    `[domain_start_spine, forth_rib]` (2, the cell's very first edge),
//!    `[back_rib, spine, forth_rib]` (3, an interior middle edge), or
//!    `[back_rib, closing_spine]` (2, the cell's un-ribbed closing edge).
//! 3. [`get_next_unconnected_chain_terminates_within_edge_count_bound`] --
//!    walking the dead-end-then-`.twin` chain (`getNextUnconnected`) from
//!    *any* starting edge terminates (returns to an already-visited start, or
//!    reaches a genuine open end) within `edge_count` steps -- asserted as an
//!    actual bounded loop that fails the test if not, not merely assumed.
//! 4. [`junction_count_delta_bound_at_domain_chain_stitches`] -- for every
//!    pair of central `NORMAL` edges directly adjacent in a domain's
//!    `connectJunctions` chain (which, per this crate's per-cell chain
//!    construction, always share a literal graph vertex -- see
//!    `build_domain_chains`'s doc comment), the two edges' own per-edge
//!    junction-fan lengths (`generate_junctions`'s `from_junctions`/
//!    `to_junctions`, which -- within a single edge -- are always equal,
//!    both driven by that edge's own "to" vertex's `bead_count`; see this
//!    file's `build_domain_chains` doc comment for the derivation) differ by
//!    at most 1. This is the observable, per-edge form of the bead-count
//!    ±1-per-transition-step guarantee `generate_transition_mids`/
//!    `apply_transitions` exist to enforce (Step 6 of this packet fixed a
//!    real multi-split-rescale-direction bug in exactly that mechanism).
//!    Checked against real `run_arachne_pipeline` output on 2 distinct
//!    fixtures (a square, and a tapered wedge), not a synthetic hand-built
//!    graph.
//!
//! # Part 2 -- `test_voronoi.cpp` degenerate-input triage (AC-8)
//!
//! `OrcaSlicerDocumented/tests/libslic3r/test_voronoi.cpp` (2163 lines, 16
//! `TEST_CASE`s) was triaged in a prior dispatch this session for cases
//! meeting the "degenerate input" criterion; 10 qualified. Per this packet's
//! own instructions, this session does **not** re-read
//! `OrcaSlicerDocumented/` directly -- the case descriptions below are the
//! one-line summaries handed down from that prior triage, cross-checked here
//! only against this crate's *own* current degeneracy-handling contract
//! (`crates/slicer-core/src/arachne/preprocess.rs`'s 9-stage
//! `preprocess_input_outline` pipeline, and
//! `crates/slicer-core/tests/voronoi_stress.rs`'s existing collinear/
//! T-junction/duplicate-vertex coverage). Without the exact source
//! coordinates, porting a fixture would mean fabricating geometry to stand
//! in for an un-read reference -- not done here; every "gap" below is a
//! recommendation for a follow-up packet that *does* read the source, not a
//! completed port.
//!
//! | # | `test_voronoi.cpp` case | Verdict | Reasoning |
//! |---|---|---|---|
//! | 1 | `:34` near-collinear sweep-line arrangement, missing edges (boost ticket 12067) | Not ported (partial structural mitigation) | `voronoi_stress_collinear` only covers *exact* collinear segments sharing an endpoint -- materially easier than a near-collinear (perturbed) sweep-line event-ordering bug inside boost itself. `preprocess_input_outline` stage 5 (`remove_colinear_edges`, `colinear_angle_tolerance_rad` = 0.005 rad default) structurally removes near-collinear *ring vertices* before they ever reach voronoi construction, which would plausibly sidestep this exact ticket's geometry class -- but no fixture proves it against ticket 12067's specific configuration. |
//! | 2 | `:82` near-self-intersecting polygon (edge split by a point), missing edges during gap-fill (ticket 12707) | Genuine gap | "Edge split by a point" is an interior *touch*, not a crossing self-intersection; `preprocess_input_outline` stages 3/6 (`fix_self_intersections` via Clipper2 `union_ex`, a `NonZero`-fill-rule reconstruction) target crossing self-intersections and are not guaranteed to react to a non-crossing near-touch. No existing test exercises this. |
//! | 3 | `:203` thin/near-degenerate polygon slivers, malformed diagram | Adequately covered | Stage 1 (triple offset) and stage 8 (`remove_small_areas`) are specifically designed to destroy sub-epsilon slivers before they ever reach voronoi construction; `triple_offset_destroys_a_sub_epsilon_square` proves the destroy mechanism directly (different geometry, same class of fix). |
//! | 4 | `:314` symmetric point set, tight spacing, division-by-zero (ticket 12903) | Inconclusive | This is boost's own point-cell math on a specific coordinate configuration. `preprocess_input_outline` stage 2's `merge_short_segments` (0.5mm default) or stage 4/7 dedup could incidentally collapse a sufficiently tight cluster, but whether this ticket's exact spacing clears that threshold is unknown without the source coordinates. |
//! | 5 | `:345` self-intersecting input loops, NaN vertex coords (ticket 12139, upstream `[!mayfail]`) | Not worth porting | Upstream itself does not guarantee this passes -- holding this crate to a stricter bar than the reference it ports would be over-engineering. Separately (informational only): production inputs only reach `SkeletalTrapezoidationGraph::from_polygons` after `preprocess_input_outline`'s own self-intersection fix-up stages, so a raw self-intersecting loop should not reach voronoi construction via the real pipeline anyway. |
//! | 6 | `:1954` collinear point lying mid-edge (T-junction), missing vertex/edge | **Genuine gap (top follow-up candidate)** | `voronoi_stress_t_junction`'s own doc comment states plainly that the *interior*-touch T-junction case is "the caller's -- T-204's -- responsibility, not this wrapper's" (T-204 = `preprocess_input_outline`, per that module's own header). Yet none of the 9 stages explicitly splits an edge at a point where another ring's vertex touches it, and no test proves this is handled. |
//! | 7 | `:1986` same mid-edge T-junction, across nested contour+hole polygons | **Genuine gap (bundle with #6)** | Same root cause as #6, for the contour/hole configuration -- more directly relevant to this project's MMU/multi-region slicing than the single-ring case. |
//! | 8 | `:2024` near-collinear points across two adjacent polygons, missing vertex | Already covered (different layer) | This is exactly the documented `D-112-MMU-TOPOLOGY`/`D-113B-WIDE-REGION-COORD-INSTABILITY` class already defended by `graph.rs`'s `clamp_implausible_vertex` -- a downstream safety net explicitly because ("the corruption is a multi-segment interaction across sites from two fragments, not a single ring's own local near-collinearity, so it does not reduce to one provably-complete pre-snap rule", `graph.rs`'s own doc comment) -- with its own regression test `from_polygons_clamps_captured_runaway_reproduction`. No `preprocess.rs` action needed. |
//! | 9 | `:2065` near-collinear closely-spaced points, duplicate output Voronoi vertices | Inconclusive (partial) | Same near-collinear input class as #1/#10 but a different failure mode (duplicate *output* vertices, not missing ones). Mitigated only if the specific spacing/angle clears one of preprocess's two thresholds (`colinear_angle_tolerance_rad` or `smallest_segment_mm`); both exist and are tested for their own generic mechanism, but neither has a fixture proving this specific failure mode is caught. |
//! | 10 | `:2108` very close/near-collinear points + a missing vertex, self-intersecting Voronoi edges | Not ported (same class as #1/#9) | A compound of the same near-collinear degeneracy already triaged at #1/#9; no new information beyond that verdict. |
//!
//! **Conclusion:** zero fixtures ported this session (would require reading
//! `OrcaSlicerDocumented/` directly, out of scope per this packet's
//! instructions). Two genuine gaps identified (#6/#7, mid-edge T-junctions),
//! explicitly named by this crate's *own* existing `voronoi_stress.rs` doc
//! comment as outside `voronoi.rs`'s contract and belonging to
//! `preprocess.rs` -- yet unaddressed there today. One case is already
//! covered by a different mechanism (#8, `graph.rs`'s defensive clamp). One
//! case is correctly excluded per upstream's own `[!mayfail]` marker (#5).
//! The remainder (#1, #3, #4, #9, #10) range from "adequately covered" to
//! "plausible mitigation exists but unproven for this exact configuration" --
//! recommend a follow-up packet with direct `OrcaSlicerDocumented` access to
//! port #6/#7 as the highest-value additions.

use std::collections::{BTreeSet, HashSet};

use slicer_core::arachne::{
    preprocess_input_outline, run_arachne_pipeline, ArachneParams, PreprocessParams,
};
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    apply_transitions, assign_bead_counts, filter_central, generate_transition_mids,
    propagate_beadings_downward, propagate_beadings_upward, CentralityParams, EdgeType,
    SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

fn mm(v: f64) -> i64 {
    (v * UNITS_PER_MM) as i64
}

/// A plain 10mm square -- large enough that its medial axis clears
/// `ArachneParams::default()`'s `optimal_width` (see
/// `crates/slicer-core/tests/arachne_pipeline.rs`'s own fixture doc comment
/// for the derivation of why a millimeter-scale square would not).
fn square_10mm() -> ExPolygon {
    let s = mm(10.0);
    expoly(vec![p(0, 0), p(s, 0), p(s, s), p(0, s)])
}

/// A 20mm x 10mm rectangle -- a "simple non-square closed shape" data point
/// for invariant 1, distinct from the square's four-fold symmetry.
fn rectangle_20x10mm() -> ExPolygon {
    let w = mm(20.0);
    let h = mm(10.0);
    expoly(vec![p(0, 0), p(w, 0), p(w, h), p(0, h)])
}

/// A symmetric trapezoid ("wedge") tapering from a 20mm-wide base at `x=0` to
/// a 2mm-wide tip at `x=40mm`: a simple, non-square, non-degenerate shape
/// whose medial axis carries genuine, monotonically-varying bead counts end
/// to end (unlike a square's four-fold-symmetric X-skeleton), giving
/// invariants 1 and 4 a second, structurally distinct fixture per this
/// packet's instructions ("square + a wedge/tapered shape").
fn wedge_trapezoid() -> ExPolygon {
    let x1 = mm(40.0);
    let half_wide = mm(10.0);
    let half_narrow = mm(1.0);
    expoly(vec![
        p(0, -half_wide),
        p(x1, -half_narrow),
        p(x1, half_narrow),
        p(0, half_wide),
    ])
}

fn simple_fixtures() -> Vec<ExPolygon> {
    vec![square_10mm(), rectangle_20x10mm(), wedge_trapezoid()]
}

// ---------------------------------------------------------------------------
// Graph-construction helpers (mirroring `arachne::pipeline`'s own stage
// order over public APIs only -- no private internals touched)
// ---------------------------------------------------------------------------

/// Builds a fully-propagated `SkeletalTrapezoidationGraph` for `poly` by
/// calling the exact same public stage functions
/// `crate::arachne::pipeline::run_arachne_pipeline` chains internally
/// (`preprocess_input_outline` -> `from_polygons` -> `filter_central` ->
/// `assign_bead_counts` -> `generate_transition_mids` -> `apply_transitions`
/// -> `propagate_beadings_upward` -> `propagate_beadings_downward`). Because
/// every one of those stages is a pure, deterministic function of its input
/// (see each stage's own module doc comment), the graph this test inspects
/// is provably the same graph `run_arachne_pipeline` itself builds for the
/// same polygon -- not a hand-tuned stand-in.
///
/// Mirrors `arachne::pipeline::to_centrality_params`'s own permissive-PI-
/// angle choice for the same documented reason: a square/rectangle/simple-
/// wedge input has no sharp/reflex corners for the rib topology to mark, so
/// a tight transitioning angle would reject every radial spine edge as
/// non-central.
fn build_propagated_graph(poly: &ExPolygon) -> SkeletalTrapezoidationGraph {
    let cleaned =
        preprocess_input_outline(std::slice::from_ref(poly), &PreprocessParams::default());
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(&cleaned)
        .expect("fixture polygon should build a skeletal graph");

    let centrality_params = CentralityParams::new(0.01 * UNITS_PER_MM, 0.0);
    filter_central(&mut graph, &centrality_params, std::f64::consts::PI);

    let strategy = BeadingStrategyFactory::create_stack(&BeadingFactoryParams::default());
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("filter_central just ran, so centrality_filtered is true");

    generate_transition_mids(&mut graph, strategy.as_ref());
    apply_transitions(&mut graph);
    propagate_beadings_upward(&mut graph);
    propagate_beadings_downward(&mut graph);

    graph
}

/// Resolves a half-edge's "to" vertex via its twin's `start_vertex`, matching
/// [`SkeletalTrapezoidationGraph`]'s own convention (duplicated locally since
/// the crate's own copies of this helper are private to their modules).
fn resolve_to_vertex(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> usize {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return NO_INDEX;
    };
    if edge.twin == NO_INDEX {
        return NO_INDEX;
    }
    graph
        .edges
        .get(edge.twin)
        .map(|twin| twin.start_vertex)
        .unwrap_or(NO_INDEX)
}

/// Walks `.next` from `start` until a dead end (`.next == NO_INDEX`),
/// returning the full edge list walked. Mirrors
/// `arachne::generate_toolpaths::find_quad`. Bounded by `graph.edges.len()`
/// steps; panics (failing the test) rather than looping forever if that
/// bound is exceeded, per invariant 3's "actual bounded-loop assertion"
/// requirement.
fn find_quad(graph: &SkeletalTrapezoidationGraph, start: usize) -> Vec<usize> {
    let max_len = graph.edges.len() + 1;
    let mut quad = vec![start];
    loop {
        assert!(
            quad.len() <= max_len,
            "quad walk from edge {start} exceeded {max_len} edges without reaching a dead end \
             (.next cycle?)"
        );
        let current = *quad.last().expect("quad always has at least one edge");
        let next = graph.edges[current].next;
        if next == NO_INDEX {
            break;
        }
        quad.push(next);
    }
    quad
}

/// Walks `.next` from `start` to its dead end and returns that dead-end
/// edge's own index. Bounded by `max_steps`; panics (failing the test)
/// rather than looping forever if exceeded.
fn walk_to_dead_end(graph: &SkeletalTrapezoidationGraph, start: usize, max_steps: usize) -> usize {
    let mut current = start;
    for _ in 0..=max_steps {
        let next = graph.edges[current].next;
        if next == NO_INDEX {
            return current;
        }
        current = next;
    }
    panic!(
        "walk_to_dead_end from edge {start} did not reach a dead end (.next == NO_INDEX) within \
         {max_steps} steps -- possible .next cycle"
    );
}

/// Builds the ordered per-domain chains of central `NORMAL` edges that
/// `connectJunctions`/`generate_toolpaths` would walk, by replicating its
/// documented algorithm (`unprocessed_quad_starts` seeded from every edge
/// with no `.prev`, `find_quad` + a `.twin`-hop off each quad's dead end)
/// directly over public graph fields.
///
/// # Why adjacent chain entries always share a literal graph vertex
///
/// Per `graph.rs`'s `make_rib`/`transfer_edge`: within one cell's own
/// per-cell chain, a rib pair's `back_edge` both starts and ends at the same
/// spine vertex (`push_edge(spine_node, node, ...)` / `push_edge(node,
/// spine_node, ...)`), and the *next* transferred spine edge in that cell's
/// chain starts from `edge_to[back_edge] == spine_node` -- the exact same
/// vertex object, not a new one at the same position. Across a cell
/// boundary, `transfer_edge`'s "already transferred" branch reuses the
/// neighbor's own vertex indices verbatim (`new_from = edge_to[twin]`) via
/// `vd_node_to_he_node` deduplication, so cross-cell adjacency is the same
/// literal-vertex-sharing guarantee. So for two central `NORMAL` edges
/// `chain[i]`/`chain[i+1]` built by this walk, `resolve_to_vertex(chain[i])
/// == chain[i+1].start_vertex` always holds by construction; the "stitch"
/// invariant this file checks is therefore precisely a per-edge property of
/// `chain[i+1]` (`|bead_count(chain[i+1].start_vertex) -
/// bead_count(resolve_to_vertex(chain[i+1]))| <= 1`), expressed as the
/// equivalent cross-edge comparison
/// `|bead_count(resolve_to_vertex(chain[i])) -
/// bead_count(resolve_to_vertex(chain[i+1]))| <= 1` to match this packet's
/// literal wording ("from_junctions.len() - to_junctions.len()", which for
/// any single edge in `generate_junctions` are always equal, both driven by
/// that edge's own "to" vertex `bead_count` -- see that function's source).
fn build_domain_chains(graph: &SkeletalTrapezoidationGraph) -> Vec<Vec<usize>> {
    let mut unprocessed: BTreeSet<usize> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.prev == NO_INDEX)
        .map(|(idx, _)| idx)
        .collect();

    let mut chains = Vec::new();
    let max_domains = graph.edges.len() + 1;
    let mut domains_processed = 0usize;

    while let Some(&domain_start) = unprocessed.iter().next() {
        domains_processed += 1;
        assert!(
            domains_processed <= max_domains,
            "domain walk exceeded {max_domains} domains without exhausting unprocessed starts"
        );

        let mut chain = Vec::new();
        let mut quad_start = domain_start;
        loop {
            if !unprocessed.remove(&quad_start) {
                break;
            }
            let quad = find_quad(graph, quad_start);
            let quad_end = *quad.last().expect("quad always has at least one edge");
            for &edge_idx in &quad {
                let edge = &graph.edges[edge_idx];
                if edge.central && edge.edge_type == EdgeType::NORMAL {
                    chain.push(edge_idx);
                }
            }
            let next_start = graph
                .edges
                .get(quad_end)
                .map(|e| e.twin)
                .unwrap_or(NO_INDEX);
            if next_start == NO_INDEX || next_start == domain_start {
                break;
            }
            if !unprocessed.contains(&next_start) {
                break;
            }
            quad_start = next_start;
        }
        chains.push(chain);
    }

    chains
}

// ---------------------------------------------------------------------------
// Invariant 1: closed-ring outer wall for simple input
// ---------------------------------------------------------------------------

/// AC-8 invariant 1: for a plain closed polygon, the outer wall
/// (`inset_idx == 0`) `ExtrusionLine`(s) produced by the full
/// `run_arachne_pipeline` are closed rings (`is_closed == true`).
#[test]
fn outer_wall_is_closed_ring_for_simple_polygons() {
    for poly in simple_fixtures() {
        let (lines, _) = run_arachne_pipeline(
            std::slice::from_ref(&poly),
            &ArachneParams::default(),
            false,
        )
        .expect("simple fixture polygon should produce Ok(lines)");

        let outer_lines: Vec<_> = lines.iter().filter(|l| l.inset_idx == 0).collect();
        assert!(
            !outer_lines.is_empty(),
            "expected at least one inset_idx == 0 (outer wall) ExtrusionLine"
        );
        for line in outer_lines {
            assert!(
                line.is_closed,
                "outer wall (inset_idx == 0) line must be a closed ring for a simple closed \
                 polygon input, got an open line with {} junctions",
                line.junctions.len()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 2: quad chains span 2-3 edges
// ---------------------------------------------------------------------------

/// AC-8 invariant 2: every domain-start edge (`.prev == NO_INDEX`) in a
/// constructed graph begins a quad (walk `.next` to the next dead end) of
/// exactly 2 or 3 edges.
#[test]
fn quad_chains_span_two_or_three_edges() {
    for poly in simple_fixtures() {
        let cleaned =
            preprocess_input_outline(std::slice::from_ref(&poly), &PreprocessParams::default());
        let graph = SkeletalTrapezoidationGraph::from_polygons(&cleaned)
            .expect("fixture polygon should build a skeletal graph");

        let domain_starts: Vec<usize> = graph
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.prev == NO_INDEX)
            .map(|(idx, _)| idx)
            .collect();
        assert!(
            !domain_starts.is_empty(),
            "expected at least one domain-start edge (.prev == NO_INDEX)"
        );

        for &start in &domain_starts {
            let quad = find_quad(&graph, start);
            assert!(
                quad.len() == 2 || quad.len() == 3,
                "quad starting at domain-start edge {start} has {} edge(s) (expected 2 or 3): \
                 {quad:?}",
                quad.len()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 3: getNextUnconnected termination bound
// ---------------------------------------------------------------------------

/// AC-8 invariant 3: walking the dead-end-then-`.twin` chain
/// (`getNextUnconnected`) from *any* starting edge terminates within
/// `edge_count` steps. Written as an actual bounded-loop assertion (fails
/// the test if not terminated by then), not merely assumed.
#[test]
fn get_next_unconnected_chain_terminates_within_edge_count_bound() {
    for poly in simple_fixtures() {
        let cleaned =
            preprocess_input_outline(std::slice::from_ref(&poly), &PreprocessParams::default());
        let graph = SkeletalTrapezoidationGraph::from_polygons(&cleaned)
            .expect("fixture polygon should build a skeletal graph");

        let edge_count = graph.edges.len();
        assert!(edge_count > 0, "expected a non-empty graph");

        for start in 0..edge_count {
            let mut current = start;
            let mut visited: HashSet<usize> = HashSet::new();
            let mut hops = 0usize;
            loop {
                assert!(
                    hops <= edge_count,
                    "getNextUnconnected chain starting at edge {start} did not terminate within \
                     {edge_count} hops (possible infinite loop)"
                );
                if !visited.insert(current) {
                    // Returned to an already-visited edge: the chain closed a
                    // loop and has terminated.
                    break;
                }
                let dead_end = walk_to_dead_end(&graph, current, edge_count + 1);
                let next = graph.edges[dead_end].twin;
                if next == NO_INDEX {
                    // Reached a genuine open end: terminated.
                    break;
                }
                current = next;
                hops += 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 4: junction-count delta bound
// ---------------------------------------------------------------------------

/// AC-8 invariant 4: `|from_junctions.len() - to_junctions.len()| <= 1` at
/// every junction stitch `connectJunctions` performs -- see
/// `build_domain_chains`'s doc comment for why this reduces to a per-edge
/// bead-count-delta check across two literally-shared-vertex-adjacent
/// central edges. Verified against real `run_arachne_pipeline` output (not a
/// synthetic hand-built graph) across 2 distinct fixtures.
///
/// # Status: BLOCKED -- genuine counter-example found, root-caused
///
/// This test currently **fails** on the `square_10mm()` fixture. This is not
/// a test-harness mistake and the assertion has deliberately **not** been
/// weakened, narrowed, or given an exclusion list to force a pass (per this
/// packet's "do not game verification" mandate) -- it is reporting a real,
/// root-caused defect in `propagation.rs::propagate_beadings_downward`
/// (packet 112 Step 3 / packet 113b Step 4; untouched by packet 113c's own
/// Step 6 fix, which was scoped to `apply_transitions` only).
///
/// **Root cause:** `propagate_beadings_downward`'s `let transition_dist =
/// 4.0;` is an admitted placeholder ("upstream uses a configured
/// beading-propagation transition distance") expressed in this crate's
/// scaled-integer unit space (1 unit = 100 nm), so `4.0` units = 0.0004 mm --
/// roughly four orders of magnitude smaller than any real edge in a
/// millimeter-scale fixture. Its "blend" branch computes `ratio_of_top =
/// edge_len / total_dist.min(transition_dist)`, which -- because
/// `transition_dist` is always the smaller operand for any realistic edge --
/// simplifies to `edge_len / 4.0`, a huge value that
/// `interpolate_bead_counts` immediately clamps to `t = 1.0`. A `t` of
/// exactly `1.0` collapses the intended interpolation
/// (`bottom_bc * (1 - t) + peak_bc * t`) to `peak_bc` outright, so *every*
/// vertex reachable this way has its own, already-correct `bead_count`
/// (assigned by `assign_bead_counts` from that vertex's own
/// `distance_to_boundary`) silently overwritten by its upward neighbor's
/// (peak) bead count instead of being blended with it.
///
/// **Concrete counter-example** (10mm square, `ArachneParams::default()`,
/// `BeadingFactoryParams::default()`): corner vertex at `(0, 0)` has
/// `distance_to_boundary == 0.0`, so `assign_bead_counts`'s primary pass
/// correctly assigns it `bead_count = Some(0)` (`DistributedBeadingStrategy::
/// optimal_bead_count(0.0) == 0`, hand-verified against
/// `crates/slicer-core/src/beading/distributed.rs`). But that same corner is
/// also the low-radius ("bottom") endpoint of a central spine edge running
/// to the square's center (`distance_to_boundary == 5mm`, `bead_count =
/// Some(9)`, the configured `max_bead_count`). `propagate_beadings_downward`
/// processes that edge, finds the corner's bead count already `Some(_)`
/// (`bottom_has_upward == true`), and -- via the clamp-to-1.0 bug above --
/// overwrites it from `0` to `9`. All four of a plain square's corners
/// exhibit this identical corruption (hand-verified via a temporary
/// diagnostic dump during this investigation, since removed). Every other
/// vertex along the same spine (the ones `apply_transitions`/`insert_node`
/// created specifically to ramp the bead count down by exactly 1 per step)
/// is *not* corrupted -- only the two chain endpoints where a `bead_count`
/// pre-existed from a source other than this same propagation pass are
/// affected, which is exactly the domain-chain "stitch" boundary invariant 4
/// checks.
///
/// **Disposition:** `propagation.rs` is read-only for packet 113c Step 8
/// (not in this step's edit whitelist), so this test intentionally reports
/// the failure rather than patching around it. Routed back for a follow-up
/// fix in `propagate_beadings_downward` (either wiring a real, unit-correct
/// `transition_dist`, e.g. `default_transition_length` converted from mm, or
/// reworking the "single edge, no quad-chain prev/next" `total_dist`
/// simplification the function's own comment already flags as incomplete).
#[test]
fn junction_count_delta_bound_at_domain_chain_stitches() {
    for poly in [square_10mm(), wedge_trapezoid()] {
        // Corroborate with real production output first, per this
        // invariant's "reading actual ExtrusionLine/junction data" mandate.
        let (lines, _) = run_arachne_pipeline(
            std::slice::from_ref(&poly),
            &ArachneParams::default(),
            false,
        )
        .expect("fixture polygon should produce Ok(lines)");
        assert!(
            !lines.is_empty(),
            "expected non-empty real toolpath output before checking the underlying graph's \
             bead-count structure"
        );

        // `preprocess_input_outline` and `SkeletalTrapezoidationGraph::from_polygons`
        // are both pure, deterministic functions of `poly` (see their own module
        // doc comments), so this independently-built graph is provably the same
        // graph `run_arachne_pipeline` used to produce `lines` above.
        let graph = build_propagated_graph(&poly);
        let chains = build_domain_chains(&graph);

        let mut checked_pairs = 0usize;
        for chain in &chains {
            for pair in chain.windows(2) {
                let (edge_a, edge_b) = (pair[0], pair[1]);
                let to_a = resolve_to_vertex(&graph, edge_a);
                let to_b = resolve_to_vertex(&graph, edge_b);
                let bc_a = graph.vertices.get(to_a).and_then(|v| v.bead_count);
                let bc_b = graph.vertices.get(to_b).and_then(|v| v.bead_count);
                if let (Some(bc_a), Some(bc_b)) = (bc_a, bc_b) {
                    checked_pairs += 1;
                    let delta = (bc_a as i64 - bc_b as i64).abs();
                    assert!(
                        delta <= 1,
                        "junction-count delta bound violated at a domain-chain stitch between \
                         edge {edge_a} (to-vertex bead_count {bc_a}) and edge {edge_b} \
                         (to-vertex bead_count {bc_b}): delta {delta} > 1"
                    );
                }
            }
        }
        assert!(
            checked_pairs > 0,
            "expected at least one adjacent central-edge pair to check in this fixture's domain \
             chains"
        );
    }
}
