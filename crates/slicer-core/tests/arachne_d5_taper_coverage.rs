//! Structural regression for the D5 tapered-wall dropout (Arachne Parity
//! Recovery campaign). Unit-INDEPENDENT COVERAGE invariant: `run_arachne_pipeline`
//! must emit walls covering essentially the full X-extent of its input polygon,
//! with no whole-region dropout.
//!
//! # The bug this pins
//!
//! A benchy hull cross-section tapers to a pointed **bow**. That bow's medial-
//! axis spine consists of segment-segment bisectors of the two converging hull
//! sides, whose `dR/dD` (≈ 0.28–1.0) legitimately exceeds the centrality cap
//! `sin(wall_transition_angle/2)` = `sin(5°)` ≈ 0.087 — so the bow is (correctly,
//! matching canonical) NON-central and receives no *primary* bead count. The
//! former `generate_junctions` gate skipped any peak whose `bead_count` was
//! `None`/`0`, which dropped every wall in the whole tapered region — the D5
//! benchy-bow dropout (the +X half of the cross-section vanished from Z≈0.4 up).
//!
//! Canonical OrcaSlicer `generateJunctions` (`SkeletalTrapezoidation.cpp:1740-1744`)
//! skips ONLY when both endpoints share an equal, non-negative bead count or the
//! edge is not upward, and synthesizes a beading from `distance_to_boundary` for
//! unassigned peaks (`getOrCreateBeading`, `:1808-1839`). This test locks in that
//! faithful behavior.
//!
//! The fixture is a real Z≈0.4 benchy cross-section captured from a live slice
//! (input bbox x[-25.76, 15.78]); the assertion is coverage-ratio only, so it is
//! invariant to PnP's absolute-unit divergence from OrcaSlicer.
#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

use slicer_core::arachne::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

fn parse_raw_contour(text: &str) -> ExPolygon {
    let mut in_raw = false;
    let mut points = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with("RAW contour") {
            in_raw = true;
            continue;
        }
        if line.starts_with("CLEANED") || line.starts_with("RAW hole") {
            in_raw = false;
            continue;
        }
        if in_raw && !line.is_empty() {
            let mut it = line.split_whitespace();
            let x: i64 = it.next().unwrap().parse().unwrap();
            let y: i64 = it.next().unwrap().parse().unwrap();
            points.push(Point2 { x, y });
        }
    }
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

fn input_x_extent(poly: &ExPolygon) -> (f64, f64) {
    let mut minx = i64::MAX;
    let mut maxx = i64::MIN;
    for p in &poly.contour.points {
        minx = minx.min(p.x);
        maxx = maxx.max(p.x);
    }
    (minx as f64 / UNITS_PER_MM, maxx as f64 / UNITS_PER_MM)
}

fn output_x_extent(lines: &[slicer_ir::ExtrusionLine]) -> Option<(f64, f64)> {
    let mut minx = f64::MAX;
    let mut maxx = f64::MIN;
    let mut any = false;
    for l in lines {
        for j in &l.junctions {
            any = true;
            minx = minx.min(j.p.x as f64);
            maxx = maxx.max(j.p.x as f64);
        }
    }
    if any {
        Some((minx, maxx))
    } else {
        None
    }
}

#[test]
fn d5_benchy_bow_cross_section_is_covered_by_arachne_walls() {
    let text = include_str!("fixtures/arachne/d5_benchy_call1.txt");
    let poly = parse_raw_contour(text);
    let (in_minx, in_maxx) = input_x_extent(&poly);

    let (lines, _inner) =
        run_arachne_pipeline(std::slice::from_ref(&poly), &ArachneParams::default(), false)
            .expect("pipeline must not error on a real benchy cross-section");

    let out = output_x_extent(&lines);
    eprintln!(
        "[d5-coverage] input X[{in_minx:.2},{in_maxx:.2}] ({:.2}mm span); output {:?}; lines={}",
        in_maxx - in_minx,
        out,
        lines.len(),
    );

    let (out_minx, out_maxx) = out.expect("arachne must emit at least some walls");
    let in_span = in_maxx - in_minx;
    let out_span = out_maxx - out_minx;
    let coverage = out_span / in_span;
    eprintln!(
        "[d5-coverage] output X[{out_minx:.2},{out_maxx:.2}] ({out_span:.2}mm span); X-coverage={:.1}%",
        coverage * 100.0
    );

    // COVERAGE INVARIANT: emitted walls must span at least 90% of the input
    // polygon's X-extent. A whole-region dropout (bow half missing) fails here.
    assert!(
        coverage >= 0.90,
        "D5 dropout: arachne walls cover only {:.1}% of the input X-extent \
         (input maxx {in_maxx:.2}mm, output maxx {out_maxx:.2}mm) — the +X region is dropped",
        coverage * 100.0
    );
}
