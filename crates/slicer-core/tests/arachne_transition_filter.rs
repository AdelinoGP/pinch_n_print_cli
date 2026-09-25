//! Bead-count transition filtering on rings (canonical
//! `SkeletalTrapezoidation::filterTransitionMids` and its helpers
//! `dissolveNearbyTransitions`, `dissolveBeadCountRegion`,
//! `filterEndOfCentralTransition`, in `SkeletalTrapezoidation.cpp`).
//!
//! A ring whose thickness hovers around a bead-count transition must not
//! print a patchwork of short transition regions. Canonical dissolves a
//! bead-count region bounded by two same-count transitions whenever the line
//! width it would have to absorb stays within `allowed_filter_deviation`
//! (the configured `wall_transition_filter_deviation`, 25% of the nozzle =
//! 0.1 mm); an even result spreads the deviation over two lines.
//!
//! # Oracle
//!
//! OrcaSlicer 2.4.1 CLI, `0.20mm Standard @BBL X1C` with Arachne, outer wall
//! 0.42 mm, inner wall 0.45 mm, `min_bead_width` 85%, `wall_loops` 5,
//! `precise_outer_wall` 1, `wall_transition_filter_deviation` 25%. Rings are
//! 1 mm tall 48-gon extrusions of outer radius 3 mm, layer z = 0.6:
//!
//! - 1.3 mm and 1.4 mm thick: 2 outer walls + 1 closed centre loop.
//! - 1.65 mm thick: 2 outer walls + 2 closed inner loops.
//! - Eccentric ring, hole offset 0.02 mm (Arachne thickness straddles the
//!   3->4 transition by about +-0.02 mm): 1 closed centre loop, no
//!   transition.
//! - Eccentric ring, hole offset 0.15 mm (straddles it by about +-0.15 mm):
//!   2 closed inner loops all the way round, the thin side included.
//!
//! PnP's own 3->4 transition thickness is not canonical's (see
//! `transition_thickness` below), so the eccentric rings here are centred on
//! the thickness PnP's strategy stack actually uses. What is asserted is
//! the behaviour on either side of the threshold, which the oracle fixes.
#![cfg(feature = "host-algos")]

use slicer_core::arachne::pipeline::{run_arachne_pipeline, ArachneParams};
use slicer_core::arachne::preprocess::{preprocess_input_outline, PreprocessParams};
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::beading::BeadingStrategy;
use slicer_core::skeletal_trapezoidation::{
    assign_bead_counts, filter_central, filter_noncentral_regions, filter_transition_mids,
    generate_transition_mids, CentralityParams, SkeletalTrapezoidationGraph,
    TRANSITION_FILTER_DIST_UNITS,
};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon, UNITS_PER_MM};
use std::collections::BTreeSet;

const RING_CENTRE_MM: f64 = 50.0;
const RING_OUTER_RADIUS_MM: f64 = 3.0;
const RING_FACETS: usize = 48;
const LAYER_HEIGHT_MM: f64 = 0.2;
const OUTER_WALL_WIDTH_MM: f64 = 0.42;
const INNER_WALL_WIDTH_MM: f64 = 0.45;

/// Canonical `Flow` spacing of a wall line (`rounded_rectangle` cross
/// section): what `PerimeterGenerator` hands `WallToolPaths` as the bead
/// widths.
fn spacing(width_mm: f64) -> f64 {
    width_mm - LAYER_HEIGHT_MM * (1.0 - std::f64::consts::FRAC_PI_4)
}

/// The oracle profile's Arachne parameters: 5 walls, `min_bead_width` 85% of
/// the 0.4 mm nozzle. Everything else keeps the canonical defaults
/// (`wall_transition_filter_deviation` 0.1 mm, `wall_transition_length`
/// 0.4 mm).
fn oracle_params() -> ArachneParams {
    ArachneParams {
        optimal_width: spacing(INNER_WALL_WIDTH_MM),
        preferred_bead_width_outer: spacing(OUTER_WALL_WIDTH_MM),
        min_bead_width: 0.34,
        max_bead_count: 10,
        ..ArachneParams::default()
    }
}

fn ngon(centre_x_mm: f64, radius_mm: f64, ccw: bool) -> Polygon {
    let mut points: Vec<Point2> = (0..RING_FACETS)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / RING_FACETS as f64;
            Point2 {
                x: ((centre_x_mm + radius_mm * a.cos()) * UNITS_PER_MM).round() as i64,
                y: ((RING_CENTRE_MM + radius_mm * a.sin()) * UNITS_PER_MM).round() as i64,
            }
        })
        .collect();
    if !ccw {
        points.reverse();
    }
    Polygon { points }
}

/// A 48-gon ring, outer radius `outer_mm`, hole radius `outer_mm -
/// thickness_mm`, hole centre shifted by `hole_offset_mm` along +x.
fn ring(outer_mm: f64, thickness_mm: f64, hole_offset_mm: f64) -> ExPolygon {
    ExPolygon {
        contour: ngon(RING_CENTRE_MM, outer_mm, true),
        holes: vec![ngon(
            RING_CENTRE_MM + hole_offset_mm,
            outer_mm - thickness_mm,
            false,
        )],
    }
}

/// The outline canonical `PerimeterGenerator::process_arachne` hands
/// `WallToolPaths` for a `thickness_mm` ring under the oracle profile:
/// `precise_outer_wall` shrinks the slice by `ext_perimeter_width -
/// ext_perimeter_spacing` on each side. The facets are regular, so the
/// mitred offset is the same 48-gon with the circumradius moved by
/// `d / cos(pi / 48)`.
fn oracle_outline(thickness_mm: f64) -> ExPolygon {
    let d = OUTER_WALL_WIDTH_MM - spacing(OUTER_WALL_WIDTH_MM);
    let shift = d / (std::f64::consts::PI / RING_FACETS as f64).cos();
    ring(
        RING_OUTER_RADIUS_MM - shift,
        thickness_mm - 2.0 * shift,
        0.0,
    )
}

/// The strategy stack `run_arachne_pipeline` builds for [`oracle_params`]
/// (only the fields that place the transitions differ from the factory
/// defaults).
fn oracle_strategy() -> Box<dyn BeadingStrategy> {
    let p = oracle_params();
    BeadingStrategyFactory::create_stack(&BeadingFactoryParams {
        optimal_width: p.optimal_width * UNITS_PER_MM,
        preferred_bead_width_outer: p.preferred_bead_width_outer * UNITS_PER_MM,
        max_bead_count: p.max_bead_count as usize,
        min_output_width: p.min_bead_width * UNITS_PER_MM,
        default_transition_length: p.wall_transition_length * UNITS_PER_MM,
        transition_filter_dist: p.transition_filter_dist * UNITS_PER_MM,
        ..BeadingFactoryParams::default()
    })
}

/// PnP's 3->4 bead transition thickness (mm) for the oracle profile. Used
/// only to place the eccentric rings' thickness band; never as an expected
/// value.
fn transition_thickness() -> f64 {
    oracle_strategy().get_transition_thickness(3) / UNITS_PER_MM
}

/// Mean ratio of the medial-axis thickness to the nominal thickness of a
/// regular 48-gon ring: between `cos(pi/48)` (edge against edge) and
/// `2cos/(1+cos)` (vertex against vertex).
const FACET_THICKNESS_RATIO: f64 = 0.9984;

/// An eccentric ring whose medial-axis thickness runs from about
/// `transition - band` to `transition + band` around the circumference.
fn straddling_ring(band_mm: f64) -> ExPolygon {
    ring(
        RING_OUTER_RADIUS_MM,
        transition_thickness() / FACET_THICKNESS_RATIO,
        band_mm,
    )
}

/// Bead counts on the central skeleton after `assign_bead_counts` and after
/// `filter_transition_mids`, running the pipeline's own stage order.
///
/// The centrality outer-edge filter is derived from the strategy
/// (`get_transition_thickness(0) / 2`), matching canonical `updateIsCentral`
/// and `run_arachne_pipeline`'s `to_centrality_params`. It is deliberately NOT
/// `wall_transition_filter_deviation` (the value passed to
/// `filter_transition_mids` below); the two differ whenever the outer width or
/// `detect_thin_wall` departs from this profile's.
fn central_bead_counts_around_the_filter(outline: &ExPolygon) -> (BTreeSet<u32>, BTreeSet<u32>) {
    let p = oracle_params();
    let cleaned =
        preprocess_input_outline(std::slice::from_ref(outline), &PreprocessParams::default());
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(&cleaned).expect("graph");
    let strategy = oracle_strategy();
    filter_central(
        &mut graph,
        &CentralityParams::new(
            strategy.get_transition_thickness(0) / 2.0,
            p.min_central_distance * UNITS_PER_MM,
        ),
        p.wall_transition_angle,
    );
    assign_bead_counts(&mut graph, strategy.as_ref()).expect("bead counts");
    filter_noncentral_regions(&mut graph, strategy.as_ref());
    let counts = |g: &SkeletalTrapezoidationGraph| -> BTreeSet<u32> {
        g.edges
            .iter()
            .filter(|e| e.central)
            .filter_map(|e| g.vertices[e.start_vertex].bead_count)
            .collect()
    };
    let before = counts(&graph);
    generate_transition_mids(&mut graph, strategy.as_ref());
    filter_transition_mids(
        &mut graph,
        strategy.as_ref(),
        TRANSITION_FILTER_DIST_UNITS,
        p.transition_filter_dist * UNITS_PER_MM,
    );
    (before, counts(&graph))
}

fn radius(line: &ExtrusionLine, centre_x_mm: f64) -> (f64, f64) {
    line.junctions
        .iter()
        .map(|j| (f64::from(j.p.x) - centre_x_mm).hypot(f64::from(j.p.y) - RING_CENTRE_MM))
        .fold((f64::MAX, f64::MIN), |(lo, hi), r| (lo.min(r), hi.max(r)))
}

/// The lines of `inset`, each required to be a closed loop.
fn closed_loops(lines: &[ExtrusionLine], inset: u32) -> Vec<&ExtrusionLine> {
    let loops: Vec<&ExtrusionLine> = lines.iter().filter(|l| l.inset_idx == inset).collect();
    for l in &loops {
        assert!(
            l.is_closed,
            "inset {inset} line with {} junctions is open: a bead-count transition \
             fragment",
            l.junctions.len()
        );
    }
    loops
}

/// Straddling the 3->4 transition by about 0.02 mm: canonical dissolves the
/// 4-bead half into 3 beads (walking up from the first transition, the
/// region deviates by far less than 0.1 mm). Oracle: the 0.02 mm eccentric
/// ring prints one closed centre loop.
///
/// Before the canonical `filterTransitionMids` port the walk was capped at
/// `wall_transition_filter_deviation` (0.1 mm) as a *distance*, nothing was
/// dissolved, and the 4-bead half printed as open transition fragments.
#[test]
fn marginal_bead_count_region_is_dissolved_into_one_count() {
    let outline = straddling_ring(0.02);
    let (before, after) = central_bead_counts_around_the_filter(&outline);
    assert_eq!(
        before,
        BTreeSet::from([3, 4]),
        "fixture must straddle the 3->4 transition before filtering"
    );
    assert_eq!(
        after,
        BTreeSet::from([3]),
        "the marginal 4-bead region must dissolve"
    );

    let (lines, _) = run_arachne_pipeline(&[outline], &oracle_params(), false).expect("pipeline");
    let inner = closed_loops(&lines, 1);
    assert_eq!(inner.len(), 1, "one centre loop, got {}", inner.len());
    assert!(inner[0].is_odd, "the centre loop is the odd bead of 3");
    assert_eq!(
        closed_loops(&lines, 0).len(),
        2,
        "outer walls on both sides"
    );
}

/// Straddling it by about 0.15 mm: dissolving the 4-bead half into 3 would
/// deviate by about 0.15 mm on one line (over the limit), while dissolving
/// the 3-bead half into 4 spreads about 0.15 mm over two lines (0.075 mm,
/// under it). Canonical therefore keeps 4 beads everywhere. Oracle: the
/// 0.15 mm eccentric ring prints two closed inner loops, thin side
/// included.
#[test]
fn bead_count_region_beyond_the_deviation_limit_is_not_dissolved() {
    let outline = straddling_ring(0.15);
    let (before, after) = central_bead_counts_around_the_filter(&outline);
    assert_eq!(
        before,
        BTreeSet::from([3, 4]),
        "fixture must straddle the 3->4 transition before filtering"
    );
    assert_eq!(
        after,
        BTreeSet::from([4]),
        "the 3-bead half must take the even count"
    );

    let (lines, _) = run_arachne_pipeline(&[outline], &oracle_params(), false).expect("pipeline");
    let inner = closed_loops(&lines, 1);
    assert_eq!(inner.len(), 2, "two inner loops, got {}", inner.len());
    assert!(
        inner.iter().all(|l| !l.is_odd),
        "4 beads have no odd centre line"
    );
}

/// Oracle bead counts on the outline canonical feeds `WallToolPaths`:
/// 1.3 mm and 1.4 mm rings print one centre loop (3 beads), a 1.65 mm ring
/// prints two inner loops (4 beads).
#[test]
fn ring_bead_counts_match_the_oracle() {
    for (thickness, inner_loops, odd) in [(1.3, 1, true), (1.4, 1, true), (1.65, 2, false)] {
        let (lines, _) =
            run_arachne_pipeline(&[oracle_outline(thickness)], &oracle_params(), false)
                .expect("pipeline");
        let inner = closed_loops(&lines, 1);
        assert_eq!(
            inner.len(),
            inner_loops,
            "{thickness} mm ring: expected {inner_loops} inner loop(s), got {}",
            inner.len()
        );
        assert!(
            inner.iter().all(|l| l.is_odd == odd),
            "{thickness} mm ring: inner loop odd-ness must be {odd}"
        );
        assert_eq!(
            closed_loops(&lines, 0).len(),
            2,
            "{thickness} mm ring: outer walls"
        );
        assert!(
            lines.iter().all(|l| l.inset_idx <= 1),
            "{thickness} mm ring: no bead beyond inset 1"
        );
    }
}

/// A ring four beads thick has its two inner beads within the stitch
/// distance (`bead_width_x`) of each other. Each already meets itself, so
/// canonical `PolylineStitcher::stitch` closes each one (the closing
/// candidate wins at ~0 mm) instead of chaining it onto the other ring.
/// Oracle (the same profile with `precise_outer_wall` 0, so the 1.5 mm ring
/// holds 4 beads): two separate closed inner loops.
///
/// Before the stitch fix the two rings came out as one "closed" line with a
/// radial chord between them.
#[test]
fn four_bead_ring_inner_beads_close_as_separate_loops() {
    let (lines, _) = run_arachne_pipeline(
        &[ring(RING_OUTER_RADIUS_MM, 1.5, 0.0)],
        &oracle_params(),
        false,
    )
    .expect("pipeline");
    let inner = closed_loops(&lines, 1);
    assert_eq!(inner.len(), 2, "two inner loops, got {}", inner.len());
    for l in inner {
        let (lo, hi) = radius(l, RING_CENTRE_MM);
        assert!(
            hi - lo < 0.05,
            "inner loop spans radii {lo:.3}..{hi:.3} mm: it jumps between the two beads"
        );
    }
}
