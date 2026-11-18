//! Deterministic controls for `H-ARACHNE-GRAPH` and `H-ARACHNE-PIPELINE`.
//!
//! Every control here is hand-built and reads no `PNP_PROPERTY_CONTEXT`. Each
//! one (a) proves the independent comparator the registry property uses
//! rejects the fault that property must catch, and (b) shows the real SUT
//! passes the same comparator on a named fixture. Injected faults live only in
//! test-owned graph/output copies and are never fed to a production kernel
//! without an explicit rejection contract.

use slicer_core::arachne::generate_toolpaths::generate_toolpaths;
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::beading::Beading;
use slicer_core::skeletal_trapezoidation::{STVertex, SkeletalTrapezoidationGraph, SktError};
use slicer_core::voronoi::{Vertex, NO_INDEX};
use slicer_ir::{
    ExPolygon, ExtrusionJunction, ExtrusionLine, Point2, Point3WithWidth, Polygon, UNITS_PER_MM,
};

use super::graph::{check_relations, GraphPopulations};
use super::pipeline::{
    check_output, reference_limits as reference_pipeline_limits, ClosedDomain, IndexRule,
    OutcomeMode,
};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

/// Whole millimetres to canonical units (1 unit = 100 nm; 1 mm = 10_000
/// units).
fn mm_units(mm: i64) -> i64 {
    (mm as f64 * UNITS_PER_MM) as i64
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

fn rectangle_4mm() -> ExPolygon {
    let side = mm_units(4);
    expoly(vec![p(0, 0), p(side, 0), p(side, side), p(0, side)])
}

/// A twin-bearing graph used by the mutation controls.
fn rectangle_graph() -> SkeletalTrapezoidationGraph {
    SkeletalTrapezoidationGraph::from_polygons(&[rectangle_4mm()])
        .expect("the 4 mm rectangle constructs")
}

fn assert_rejected(label: &str, graph: &SkeletalTrapezoidationGraph, expected_fragment: &str) {
    match check_relations(graph) {
        Ok(populations) => {
            panic!("{label}: the relation checker accepted the mutant {populations:?}")
        }
        Err(message) => assert!(
            message.contains(expected_fragment),
            "{label}: diagnostic {message:?} does not mention {expected_fragment:?}"
        ),
    }
}

/// AC-2 controls: real constructor rejection boundaries and an independent
/// relation checker that rejects each injected topology/table fault.
pub fn graph_controls_and_input_errors() {
    // ── Real constructor error boundaries ──────────────────────────────────
    assert_eq!(
        SkeletalTrapezoidationGraph::from_polygons(&[]),
        Err(SktError::EmptyInput),
        "empty input must return SktError::EmptyInput"
    );
    let empty_ring = ExPolygon {
        contour: Polygon { points: Vec::new() },
        holes: Vec::new(),
    };
    match SkeletalTrapezoidationGraph::from_polygons(&[empty_ring]) {
        Err(SktError::DegeneratePolygon(message)) => assert!(
            !message.is_empty(),
            "the DegeneratePolygon diagnostic must name the ring"
        ),
        other => panic!("a zero-point ring must return SktError::DegeneratePolygon, got {other:?}"),
    }
    let short_ring = expoly(vec![p(0, 0), p(mm_units(10), 0)]);
    match SkeletalTrapezoidationGraph::from_polygons(&[short_ring]) {
        Err(SktError::DegeneratePolygon(message)) => assert!(
            !message.is_empty(),
            "the DegeneratePolygon diagnostic must name the ring"
        ),
        other => panic!("a two-point ring must return SktError::DegeneratePolygon, got {other:?}"),
    }

    // ── The real SUT passes the independent checker ────────────────────────
    let graph = rectangle_graph();
    let populations: GraphPopulations =
        check_relations(&graph).expect("the real rectangle graph satisfies every relation");
    assert!(
        populations.vertices > 0 && populations.edges > 0,
        "the rectangle graph must be nonempty, got {populations:?}"
    );
    assert!(
        populations.twins > 0 && populations.resolved_endpoints == populations.twins,
        "every twin must resolve its endpoint, got {populations:?}"
    );

    // ── Injected relation faults are rejected ──────────────────────────────
    let mut broken_twin = graph.clone();
    let edge = broken_twin
        .edges
        .iter()
        .position(|edge| edge.twin != NO_INDEX)
        .expect("the rectangle has twin-bearing edges");
    let twin = broken_twin.edges[edge].twin;
    broken_twin.edges[twin].twin = twin; // self-twin instead of `edge`
    assert_rejected(
        "broken twin reciprocity",
        &broken_twin,
        "twin reciprocity broken",
    );

    let mut out_of_range_next = graph.clone();
    out_of_range_next.edges[0].next = out_of_range_next.edges.len();
    assert_rejected(
        "out-of-range next",
        &out_of_range_next,
        "neither NO_INDEX nor < edges.len()",
    );

    let mut out_of_range_start = graph.clone();
    out_of_range_start.edges[0].start_vertex = out_of_range_start.vertices.len();
    assert_rejected(
        "out-of-range start_vertex",
        &out_of_range_start,
        "neither NO_INDEX nor < vertices.len()",
    );

    let mut truncated_beading = graph.clone();
    truncated_beading.beading_propagation.pop();
    assert_rejected(
        "truncated beading_propagation",
        &truncated_beading,
        "beading_propagation.len()",
    );

    let mut truncated_distance = graph.clone();
    truncated_distance.beading_dist_to_bottom_source.pop();
    assert_rejected(
        "truncated beading_dist_to_bottom_source",
        &truncated_distance,
        "beading_dist_to_bottom_source.len()",
    );

    // The unmutated graph still passes after all copies were made.
    check_relations(&graph).expect("the original graph is untouched");
}

// ─────────────────────────────────────────────────────────────────────────────
// H-ARACHNE-PIPELINE controls.
// ─────────────────────────────────────────────────────────────────────────────

fn junction(x: f32, y: f32, width: f32, perimeter_index: u32) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y,
            width,
            // exhaustive: every non-watched geometric field is the audited
            // zero/one default for this 2D marker fixture
            ..Point3WithWidth::default()
        },
        perimeter_index,
    }
}

fn line(
    junctions: Vec<ExtrusionJunction>,
    inset_idx: u32,
    is_odd: bool,
    is_closed: bool,
) -> ExtrusionLine {
    ExtrusionLine {
        junctions,
        inset_idx,
        is_odd,
        is_closed,
    }
}

fn printable_line() -> ExtrusionLine {
    line(
        vec![
            junction(0.0, 0.0, 0.4, 0),
            junction(1.0, 0.0, 0.4, 0),
            junction(1.0, 1.0, 0.4, 0),
        ],
        0,
        false,
        false,
    )
}

fn inner_contour_marker() -> ExtrusionLine {
    line(
        vec![junction(0.0, 0.0, 0.0, 1), junction(1.0, 0.0, 0.0, 1)],
        1,
        true,
        false,
    )
}

fn assert_output_rejected(
    label: &str,
    result: Result<impl std::fmt::Debug, String>,
    fragment: &str,
) {
    match result {
        Ok(value) => panic!("{label}: the comparator accepted the mutant {value:?}"),
        Err(message) => assert!(
            message.contains(fragment),
            "{label}: diagnostic {message:?} does not mention {fragment:?}"
        ),
    }
}

/// AC-3 / AC-N2 controls: the independent output comparator catches false
/// closure in the explicit-repeat domain, a corrupted junction index and a
/// nonzero-first-width inner-contour marker.
pub fn pipeline_controls() {
    let limits = reference_pipeline_limits();

    // ── A controlled valid pair passes ─────────────────────────────────────
    let stats = check_output(
        &[printable_line()],
        &[inner_contour_marker()],
        OutcomeMode::Guaranteed,
        IndexRule::ControlledUniform,
        ClosedDomain::Pipeline,
        &limits.graph,
    )
    .expect("the controlled printable line and marker satisfy the comparator");
    assert_eq!(stats.printable_lines, 1);
    assert_eq!(stats.inner_contour_lines, 1);
    assert_eq!(stats.uniform_lines, 1);

    // ── (a) false closure in the explicit-repeat domain ────────────────────
    let closed = line(
        vec![
            junction(0.0, 0.0, 0.4, 0),
            junction(1.0, 0.0, 0.4, 0),
            junction(0.0, 0.0, 0.4, 0),
        ],
        0,
        false,
        true,
    );
    check_output(
        &[closed.clone()],
        &[],
        OutcomeMode::AllowedEmpty,
        IndexRule::ControlledUniform,
        ClosedDomain::ExplicitRepeat,
        &limits.graph,
    )
    .expect("a closed line with an explicit repeated endpoint is valid in the repeat domain");

    let mut falsely_closed = closed;
    let last = falsely_closed.junctions.len() - 1;
    falsely_closed.junctions[last].p.x = 2.0;
    assert_output_rejected(
        "falsely closed line with unequal endpoints",
        check_output(
            &[falsely_closed],
            &[],
            OutcomeMode::AllowedEmpty,
            IndexRule::ControlledUniform,
            ClosedDomain::ExplicitRepeat,
            &limits.graph,
        ),
        "is_closed but its endpoints differ",
    );

    // ── (b) a corrupted perimeter/junction index ───────────────────────────
    let mut corrupted_index = printable_line();
    corrupted_index.junctions[1].perimeter_index = 5;
    assert_output_rejected(
        "corrupted perimeter_index",
        check_output(
            &[corrupted_index],
            &[],
            OutcomeMode::AllowedEmpty,
            IndexRule::ControlledUniform,
            ClosedDomain::Pipeline,
            &limits.graph,
        ),
        "perimeter_index",
    );

    // The pipeline-domain rule is weaker, but it still rejects a line whose
    // inset index appears at neither endpoint.
    let orphaned_inset = line(
        vec![
            junction(0.0, 0.0, 0.4, 3),
            junction(1.0, 0.0, 0.4, 0),
            junction(1.0, 1.0, 0.4, 4),
        ],
        0,
        false,
        false,
    );
    assert_output_rejected(
        "inset index carried only by an interior junction",
        check_output(
            &[orphaned_inset],
            &[],
            OutcomeMode::AllowedEmpty,
            IndexRule::PipelineEndpoints,
            ClosedDomain::Pipeline,
            &limits.graph,
        ),
        "not carried by either endpoint",
    );

    // ── (c) a nonzero-first-width inner-contour marker ─────────────────────
    let mut corrupted_marker = inner_contour_marker();
    corrupted_marker.junctions[0].p.width = 0.2;
    assert_output_rejected(
        "nonzero-first-width inner-contour marker",
        check_output(
            &[],
            &[corrupted_marker],
            OutcomeMode::AllowedEmpty,
            IndexRule::ControlledUniform,
            ClosedDomain::Pipeline,
            &limits.graph,
        ),
        "zero-width marker",
    );

    // ── An empty guaranteed-output result is not printable coverage ────────
    assert_output_rejected(
        "empty guaranteed output",
        check_output(
            &[],
            &[],
            OutcomeMode::Guaranteed,
            IndexRule::ControlledUniform,
            ClosedDomain::Pipeline,
            &limits.graph,
        ),
        "no printable lines",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-3 / AC-N2: the canonical pre-stitch local-maximum line is open.
// ─────────────────────────────────────────────────────────────────────────────

/// Canonical `SkeletalTrapezoidation::generateLocalMaximaSingleBeads`
/// (`SkeletalTrapezoidation.cpp`) constructs an odd, six-junction
/// `ExtrusionLine` whose constructor starts it **open**; it never appends the
/// first junction. This calls the real public `generate_toolpaths` and inspects
/// its raw buckets, before `stitch_extrusions` can close the micro-loop.
///
/// A one-vertex graph is a valid bounded witness: `is_local_maximum`
/// (`crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`) accepts a
/// vertex with `distance_to_boundary > EPS` and no outgoing edges, and the
/// `!isCentral` gate passes vacuously on an edgeless graph.
pub fn local_maximum_pre_stitch_is_open() {
    let mut graph = SkeletalTrapezoidationGraph::default();
    graph.vertices.push(STVertex {
        // exhaustive: STVertex is not a watched literal type (4 fields)
        position: Vertex {
            x: 2_000.0,
            y: 1_000.0,
        },
        distance_to_boundary: 1_500.0,
        bead_count: Some(1),
        transition_ratio: 0.0,
    });
    graph.beading_propagation = vec![Some(Beading {
        total_thickness: 3_000.0,
        bead_widths: vec![3_000.0],
        toolpath_locations: vec![1_500.0],
        left_over: 0.0,
    })];
    graph.beading_dist_to_bottom_source = vec![0.0];

    let strategy = BeadingStrategyFactory::create_stack(&BeadingFactoryParams::default());
    let buckets = generate_toolpaths(&graph, strategy.as_ref());
    assert_eq!(
        buckets.len(),
        1,
        "one odd local maximum must populate exactly one inset bucket, got {}",
        buckets.len()
    );
    assert_eq!(buckets[0].len(), 1, "one local maximum must emit one line");
    let line = &buckets[0][0];

    assert!(line.is_odd, "the local-maximum line must be odd");
    assert_eq!(line.inset_idx, 0, "the middle bead of a one-bead peak is 0");
    assert_eq!(
        line.junctions.len(),
        6,
        "canonical emits exactly six hexagon junctions (no repeated endpoint), got {}",
        line.junctions.len()
    );
    assert!(
        !line.is_closed,
        "the raw pre-stitch local-maximum line must be open; only the stitcher may close it"
    );
    for (index, junction) in line.junctions.iter().enumerate() {
        assert_eq!(
            junction.perimeter_index, line.inset_idx,
            "junction {index} must carry the middle-bead index"
        );
        assert!(
            junction.p.x.is_finite()
                && junction.p.y.is_finite()
                && junction.p.width.is_finite()
                && junction.p.width > 0.0,
            "junction {index} must be finite with a positive width"
        );
    }
    let first = line.junctions[0].p;
    let last = line.junctions[line.junctions.len() - 1].p;
    assert!(
        first.x != last.x || first.y != last.y,
        "the six hexagon junctions must not include a repeated closing endpoint"
    );
}
