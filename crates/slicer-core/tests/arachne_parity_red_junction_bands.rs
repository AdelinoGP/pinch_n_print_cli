//! Red tests encoding finding **N1** of the second-pass Arachne parity audit
//! (`target/arachne_parity_audit_20260706_020657.md`, §N1).
//!
//! **Finding N1:** PNP's `generate_junctions`
//! (`crates/slicer-core/src/arachne/generate_toolpaths.rs:192-334`) gates on
//! `edge.central`, excludes `EXTRA_VD` rib edges, processes BOTH half-edges
//! of every central twin pair, emits ALL bead indices `0..bead_count` per
//! edge, and clamps out-of-band beads onto the edge endpoints
//! (`.clamp(0.0, 1.0)`, lines 288-297; constant-radius fallback
//! `t_from = 0, t_to = 1`).
//!
//! Canonical OrcaSlicer `generateJunctions`
//! (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2013-2079`)
//! instead iterates ALL edges with no centrality gate (ribs included), skips
//! non-upward half-edges (`from.R > to.R`, line 2017), skips flat edges and
//! edges whose endpoints share the same resolved bead count (lines
//! 2024-2027), and emits ONLY beads whose `toolpath_locations[idx]` lies
//! within the edge's `[end_R, start_R]` radius band (loop starts at the
//! middle bead index, line 2046, and breaks once `bead_R < end_R`, line
//! 2068). Out-of-band beads are skipped — never clamped.
//!
//! Two observable consequences are encoded here:
//!
//! 1. On a flat-spine polygon (a rectangle), PNP's constant-radius fallback
//!    puts outer-wall (inset 0) junctions ON THE MEDIAL AXIS (2 mm from the
//!    boundary of a 20×4 mm rectangle) instead of at the outer bead's
//!    ~0.2 mm offset.
//! 2. Because both half-edges of every central pair carry full fans, each
//!    physical bead polyline is emitted once per direction — the total
//!    inset-0 polyline length for a simple square is ~2× the single
//!    canonical ring.
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon, UNITS_PER_MM};

fn p_mm(x_mm: f64, y_mm: f64) -> Point2 {
    Point2 {
        x: (x_mm * UNITS_PER_MM) as i64,
        y: (y_mm * UNITS_PER_MM) as i64,
    }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// All inset-0 (outermost wall) lines from a pipeline run.
fn inset0_lines(lines: &[ExtrusionLine]) -> Vec<&ExtrusionLine> {
    lines.iter().filter(|l| l.inset_idx == 0).collect()
}

/// Distance (mm) from an interior point to the boundary of the axis-aligned
/// rectangle `[0, w] × [0, h]`. Negative for points outside (which still
/// count as "near the boundary" for this test's purposes).
fn dist_to_rect_boundary_mm(x: f64, y: f64, w: f64, h: f64) -> f64 {
    x.min(w - x).min(y).min(h - y)
}

/// N1, consequence 1 — flat-spine junction placement.
///
/// A 20×4 mm rectangle's medial axis has a long constant-radius (R = 2 mm)
/// horizontal spine. Canonically that flat spine edge carries NO junctions
/// (`SkeletalTrapezoidation.cpp:2024-2027` skips flat edges); the outermost
/// wall's junctions ride the rib edges at the outer bead's target radius
/// (≈ `preferred_bead_width_outer / 2` = 0.2 mm from the boundary). Under
/// PNP's constant-radius fallback (`generate_toolpaths.rs:288-297`), the
/// flat spine edge contributes inset-0 junctions AT its own endpoints —
/// 2 mm from the boundary, on the medial axis.
///
/// Asserts every inset-0 junction lies within 0.6 mm of the rectangle
/// boundary (3× the canonical 0.2 mm placement — generous slack for corner
/// geometry and preprocessing offsets). FAILS on current code.
#[test]
fn n1_rectangle_outer_wall_junctions_stay_near_boundary() {
    const W_MM: f64 = 20.0;
    const H_MM: f64 = 4.0;
    let rect = expoly(vec![
        p_mm(0.0, 0.0),
        p_mm(W_MM, 0.0),
        p_mm(W_MM, H_MM),
        p_mm(0.0, H_MM),
    ]);

    let lines = run_arachne_pipeline(
        std::slice::from_ref(&rect),
        &ArachneParams::default(),
        false,
    )
    .expect("20x4mm rectangle should produce Ok(lines)");

    let outer = inset0_lines(&lines);
    assert!(
        !outer.is_empty(),
        "expected at least one inset-0 (outer wall) line for a 20x4mm rectangle"
    );

    let mut worst: f64 = 0.0;
    let mut worst_pos = (0.0_f64, 0.0_f64);
    for line in &outer {
        for j in &line.junctions {
            let d = dist_to_rect_boundary_mm(j.p.x as f64, j.p.y as f64, W_MM, H_MM);
            if d > worst {
                worst = d;
                worst_pos = (j.p.x as f64, j.p.y as f64);
            }
        }
    }

    assert!(
        worst <= 0.6,
        "outer-wall (inset 0) junction at ({:.3}, {:.3}) mm sits {:.3} mm from the rectangle \
         boundary — canonical generateJunctions places outer-wall junctions at the outer bead's \
         target radius (~0.2 mm; SkeletalTrapezoidation.cpp:2064-2077, in-band beads only). A \
         value near 2.0 mm means a flat central spine edge emitted clamped junctions on the \
         medial axis (generate_toolpaths.rs:288-297, finding N1)",
        worst_pos.0,
        worst_pos.1,
        worst
    );
}

/// N1, consequence 2 — the outer wall sits at the wrong radius.
///
/// Canonically the outermost bead's toolpath location is
/// `preferred_bead_width_outer / 2` = 0.2 mm from the boundary: each
/// junction is placed where the LOCAL beading's `toolpath_locations[0]`
/// radius crosses the carrier edge's radius band
/// (`SkeletalTrapezoidation.cpp:2064-2077`), with the beading resolved per
/// node via `getBeading`/`BeadingPropagation` (:2091-2127). A 10 mm square
/// therefore yields an inset-0 ring of perimeter ≈ 4 × (10 − 2×0.2) =
/// 38.4 mm.
///
/// PNP resolves ONE beading per edge endpoint from that vertex's own
/// `bead_count` (`generate_toolpaths.rs:220-223`) — on a square every spine
/// endpoint away from the corners carries the domain-max bead count (9),
/// whose equal-spread `toolpath_locations[0]` lands at ~0.55 mm — so the
/// emitted outer ring measures ~32.1 mm (inset ≈ 0.5 mm): the part's outer
/// wall is pulled ~0.3 mm inward everywhere. This test pins the canonical
/// junction-radius invariant on every inset-0 junction. FAILS on current
/// code (junctions at ~0.5 mm).
#[test]
fn n1_square_outer_wall_junctions_at_outer_bead_radius() {
    const SIDE_MM: f64 = 10.0;
    let square = expoly(vec![
        p_mm(0.0, 0.0),
        p_mm(SIDE_MM, 0.0),
        p_mm(SIDE_MM, SIDE_MM),
        p_mm(0.0, SIDE_MM),
    ]);

    let lines = run_arachne_pipeline(
        std::slice::from_ref(&square),
        &ArachneParams::default(),
        false,
    )
    .expect("10mm square should produce Ok(lines)");

    let outer = inset0_lines(&lines);
    assert!(
        !outer.is_empty(),
        "expected at least one inset-0 (outer wall) line for a 10mm square"
    );

    // Canonical outer-bead toolpath radius: preferred_bead_width_outer / 2 =
    // 0.2mm. Allow [0.1, 0.35] mm — generous slack for corner rounding and
    // transition dents, but well below the ~0.5mm the domain-max-beading
    // placement produces.
    let mut worst = 0.0_f64;
    let mut worst_pos = (0.0_f64, 0.0_f64);
    for line in &outer {
        for j in &line.junctions {
            let d = dist_to_rect_boundary_mm(j.p.x as f64, j.p.y as f64, SIDE_MM, SIDE_MM);
            let err = (d - 0.2).abs();
            if err > worst {
                worst = err;
                worst_pos = (j.p.x as f64, j.p.y as f64);
            }
        }
    }

    assert!(
        worst <= 0.15,
        "inset-0 junction at ({:.3}, {:.3}) mm deviates {:.3} mm from the canonical outer-bead \
         radius (0.2 mm from the boundary = preferred_bead_width_outer / 2). Canonical \
         generateJunctions places each junction where the local beading's toolpath_locations[0] \
         crosses the carrier edge's radius band (SkeletalTrapezoidation.cpp:2064-2077, beading \
         via getBeading :2091-2127); PNP instead interpolates against per-endpoint beadings \
         computed from the endpoint's own (domain-max) bead_count \
         (generate_toolpaths.rs:220-223), pulling the whole outer wall ~0.3 mm inward \
         (finding N1)",
        worst_pos.0,
        worst_pos.1,
        worst
    );
}
