//! Reproduction harness for the long-standing "missing/merged outer walls" bug.
//!
//! An annulus (region + hole) exercises the same skeletal-topology case the
//! benchy hull hits at Z=9.6/17.2/20.6: the medial axis connects the OUTER
//! contour to the INNER hole contour through 3-way spine vertices. A faithful
//! Arachne `connectJunctions`+`addToolpathSegment` must emit the outer wall
//! and the hole's outer wall as SEPARATE `ExtrusionLine`s (split at the 3-way
//! odd vertices); the current flatten-then-emit-one-line-per-bead walk
//! concatenates them into one wrong open path, dropping the real hull outline.
//!
//! This is NOT a self-captured baseline: it asserts the geometrically correct
//! property (separate closed loops for outer contour and hole), which is what
//! OrcaSlicer produces and what the benchy needs.
//!
//! Both loops live at `inset_idx == 0` — canonical seeds bead index 0 from
//! every boundary polygon independently, contour and hole alike; see the test
//! body for the canonical citations.
//!
//! This file asserted the opposite ("inset 0 == one closed loop") until
//! 2026-07-16: its harness passed `stitch_extrusions` a `max_gap` of `0.4 *
//! UNITS_PER_MM` = 4000mm against this 2mm annulus, which stitched across the
//! 0.5mm wall and merged the two contours into one loop — so the test demanded
//! the very merge its header calls a bug. Keep `max_gap` in mm, as production
//! passes it (D-147-STITCH-TINY-POLY-UNITS).

#![cfg(feature = "host-algos")]

use slicer_core::arachne::generate_toolpaths;
use slicer_core::arachne::stitch::stitch_extrusions;
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    apply_transitions, assign_bead_counts, filter_central, generate_transition_mids,
    propagate_beadings_downward, propagate_beadings_upward, CentralityParams,
    SkeletalTrapezoidationGraph,
};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

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
        ..Default::default()
    }
}

fn centrality_params() -> CentralityParams {
    CentralityParams::new(200.0, 50.0)
}
fn run_pipeline(poly: &ExPolygon) -> Vec<ExtrusionLine> {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");
    let mut centrality_params = centrality_params();
    centrality_params.transition_filter_dist *= 0.01;
    filter_central(&mut graph, &centrality_params, std::f64::consts::PI);
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    assign_bead_counts(&mut graph, strategy.as_ref()).expect("assign");
    generate_transition_mids(&mut graph, strategy.as_ref());
    apply_transitions(&mut graph);
    propagate_beadings_upward(&mut graph);
    propagate_beadings_downward(&mut graph);
    let lines = generate_toolpaths(&graph, strategy.as_ref());
    // Flatten buckets (generate_toolpaths returns one bucket per bead index),
    // then run the SAME stitch pass the production pipeline uses so per-quad
    // rungs rejoin into closed loops and spine-separated contours stay apart.
    let flat: Vec<ExtrusionLine> = lines.into_iter().flatten().collect();
    // `max_gap` in mm, matching the production call site
    // (`arachne/pipeline.rs`: `preferred_bead_width_outer - 1e-6`) and
    // `stitch_extrusions`'s documented mm contract. Corrected 2026-07-16
    // (D-147-CHAIN-CLOSURE) from `0.4 * UNITS_PER_MM` — 4000mm of stitch
    // slack against a 2mm annulus, which joined endpoints unconditionally.
    stitch_extrusions(flat, 0.4)
}

/// Outer square (2mm) with a centred square hole (1mm) — same topology as the
/// benchy hull + hollow-interior at the broken layers.
fn annulus() -> ExPolygon {
    let outer = Polygon {
        points: vec![
            p(-10_000, -10_000),
            p(10_000, -10_000),
            p(10_000, 10_000),
            p(-10_000, 10_000),
        ],
    };
    let hole = Polygon {
        points: vec![
            p(-5_000, -5_000),
            p(5_000, -5_000),
            p(5_000, 5_000),
            p(-5_000, 5_000),
        ],
    };
    ExPolygon {
        contour: outer,
        holes: vec![hole],
    }
}

#[test]
fn annulus_outer_and_hole_are_separate_closed_loops() {
    let out = run_pipeline(&annulus());
    // Canonical inset numbering for a region with a hole: inset 0 holds BOTH
    // the outer contour's outermost bead AND the hole's outermost bead, as two
    // SEPARATE closed lines. `SkeletalTrapezoidation`'s graph construction
    // seeds `distance_to_boundary = 0` on every boundary polygon, contour and
    // hole alike, so `generateJunctions` derives bead index 0 independently per
    // boundary and `addToolpathSegment` buckets many disjoint loops per inset.
    // `PerimeterGenerator.cpp::traverse_extrusions` confirms it downstream:
    // `is_external = inset_idx == 0` alone selects `erExternalPerimeter` — the
    // struct's separate contour-vs-hole flag plays no part — so a hole's
    // outermost wall is emitted as "Outer wall" exactly like the contour's.
    //
    // The two loops must remain SEPARATE and must never be concatenated into
    // one path spanning both contours (the benchy hull "missing walls" bug).
    let mut by_inset: std::collections::BTreeMap<u32, Vec<&ExtrusionLine>> =
        std::collections::BTreeMap::new();
    for l in &out {
        by_inset.entry(l.inset_idx).or_default().push(l);
    }
    let total_closed: usize = by_inset
        .values()
        .map(|v| v.iter().filter(|l| l.is_closed).count())
        .sum();
    for (k, v) in &by_inset {
        println!(
            "inset{}: lines={} closed={} sizes={:?}",
            k,
            v.len(),
            v.iter().filter(|l| l.is_closed).count(),
            v.iter().map(|l| l.junctions.len()).collect::<Vec<_>>()
        );
    }
    // Inset 0 holds TWO separate closed loops: the outer contour's outermost
    // bead and the HOLE's outermost bead. Both boundaries seed bead index 0
    // independently, so insets march inward from each and meet at the medial
    // axis — see this test's own doc comment for the canonical basis.
    let outer = by_inset.get(&0).expect("inset 0 must exist");
    let outer_closed = outer.iter().filter(|l| l.is_closed).count();
    assert_eq!(
        outer_closed, 2,
        "inset 0 must be exactly two closed loops (outer contour + hole); got {outer_closed}"
    );
    // Both must be present as separate closed loops (never merged, never dropped).
    assert!(
        total_closed >= 2,
        "outer and hole must both be present as separate closed loops; got {total_closed}"
    );
    // No open/merged fragment in inset 0: every line there is one of the two loops.
    assert!(
        outer.len() == outer_closed,
        "inset 0 must contain only the two closed loops, no merged or open fragment"
    );

    // The anti-merge invariant this file exists to enforce, stated
    // structurally (ADR-0042 — unit-independent, no absolute coordinates):
    // one inset-0 loop must strictly ENCLOSE the other. A concatenation of
    // the outer contour with the hole (the "missing/merged outer walls" bug)
    // spans both boundaries, so its bbox would coincide with the outer
    // contour's rather than nest inside it.
    let bbox = |l: &ExtrusionLine| {
        l.junctions.iter().fold(
            (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
            |(x0, y0, x1, y1), j| (x0.min(j.p.x), y0.min(j.p.y), x1.max(j.p.x), y1.max(j.p.y)),
        )
    };
    let closed0: Vec<_> = outer.iter().filter(|l| l.is_closed).collect();
    let (a, b) = (bbox(closed0[0]), bbox(closed0[1]));
    let (inner, outer_bb) = if (a.2 - a.0) < (b.2 - b.0) {
        (a, b)
    } else {
        (b, a)
    };
    assert!(
        inner.0 > outer_bb.0
            && inner.1 > outer_bb.1
            && inner.2 < outer_bb.2
            && inner.3 < outer_bb.3,
        "the hole's inset-0 loop must nest strictly inside the outer contour's \
         inset-0 loop (never concatenated into one span): inner={inner:?} outer={outer_bb:?}"
    );
}
